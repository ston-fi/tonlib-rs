use std::collections::HashMap;
use std::sync::{Arc, Weak};
use std::time::Duration;

use futures::future::join_all;
use futures::join;
use moka::future::Cache;

use crate::address::TonAddress;
use crate::client::{
    BlockStream, BlockStreamItem, TonBlockFunctions, TonClient, TonClientError, TonClientInterface,
};
use crate::contract::TonContractError::CacheError;
use crate::contract::{TonContractError, TonContractState};
use crate::tl::{InternalTransactionId, RawFullAccountState};

type ContractStateCache = Cache<TonAddress, TonContractState>;
type TxIdCache = Cache<TonAddress, InternalTransactionId>;
type AccountStateCache = Cache<TonAddress, RawFullAccountState>;

const DELAY_ON_TON_FAILURE: u64 = 100;
#[derive(Clone)]
pub struct ContractFactoryCache {
    inner: Arc<Inner>,
}

impl ContractFactoryCache {
    pub async fn new(
        client: &TonClient,
        capacity: u64,
        time_to_live: Duration,
    ) -> Result<ContractFactoryCache, TonContractError> {
        let inner = Inner {
            client: client.clone(),
            contract_state_cache: Cache::builder()
                .max_capacity(capacity)
                .time_to_live(time_to_live)
                .build(),
            tx_id_cache: Cache::builder()
                .max_capacity(capacity)
                .time_to_live(time_to_live)
                .build(),
            account_state_cache: Cache::builder()
                .max_capacity(capacity)
                .time_to_live(time_to_live)
                .build(),
        };

        let arc_inner = Arc::new(inner);
        let weak_inner = Arc::downgrade(&arc_inner);

        tokio::task::spawn(async move { Self::run_loop(weak_inner).await });

        let cache = ContractFactoryCache { inner: arc_inner };
        Ok(cache)
    }

    pub async fn get_account_state(
        &self,
        address: &TonAddress,
    ) -> Result<RawFullAccountState, TonContractError> {
        let state_result = self
            .inner
            .account_state_cache
            .try_get_with(
                address.clone(),
                Self::load_account_state(&self.inner.client, &self.inner.tx_id_cache, address),
            )
            .await;
        match state_result {
            Ok(state) => Ok(state),
            Err(e) if e.is_transaction_hash_mismatch() => {
                // Fallback to raw.getAccountState without caching to work around bug in tonlib
                Ok(self.inner.client.get_raw_account_state(address).await?)
            }
            Err(e) => Err(CacheError(e.clone())),
        }
    }

    async fn load_account_state(
        client: &TonClient,
        tx_id_cache: &TxIdCache,
        address: &TonAddress,
    ) -> Result<RawFullAccountState, TonContractError> {
        let maybe_tx_id = tx_id_cache.get(&address).await;
        let state = if let Some(tx_id) = maybe_tx_id {
            client
                .get_raw_account_state_by_transaction(address, &tx_id)
                .await?
        } else {
            client.get_raw_account_state(address).await?
        };
        Ok(state)
    }

    pub(crate) async fn get_contract_state(
        &self,
        address: &TonAddress,
    ) -> Result<TonContractState, TonContractError> {
        let state_result = self
            .inner
            .contract_state_cache
            .try_get_with(
                address.clone(),
                Self::load_contract_state(&self.inner.client, &self.inner.tx_id_cache, address),
            )
            .await;
        match state_result {
            Ok(state) => Ok(state),
            Err(e) if e.is_transaction_hash_mismatch() => {
                // Fallback to smc.load without caching to work around bug in tonlib
                TonContractState::load(&self.inner.client, address).await
            }
            Err(e) => Err(CacheError(e.clone())),
        }
    }

    async fn load_contract_state(
        client: &TonClient,
        tx_id_cache: &TxIdCache,
        address: &TonAddress,
    ) -> Result<TonContractState, TonContractError> {
        let maybe_tx_id = tx_id_cache.get(&address).await;
        let state = if let Some(tx_id) = maybe_tx_id {
            TonContractState::load_by_transaction(client, address, &tx_id).await?
        } else {
            TonContractState::load(client, address).await?
        };
        Ok(state)
    }

    async fn run_loop(weak_inner: Weak<Inner>) {
        let mut block_stream = loop {
            if let Some(inner) = weak_inner.upgrade() {
                let client = &inner.client;
                let masterchain_info_result = client.get_masterchain_info().await;
                match masterchain_info_result {
                    Ok((_, info)) => {
                        let first_block_seqno = info.last.seqno;
                        let block_stream = BlockStream::new(&client, first_block_seqno);
                        break block_stream;
                    }
                    Err(e) => {
                        log::warn!(
                            "[ContractFactoryCache] Could not retrieve current block: {:?}",
                            e
                        );
                        tokio::time::sleep(Duration::from_millis(DELAY_ON_TON_FAILURE)).await;
                    }
                }
            } else {
                log::info!(
                    "[ContractFactoryCache] Exiting run loop before initializing BlockStream"
                );
                return;
            };
        };

        loop {
            // Must exit run loop if inner has been dropped
            if weak_inner.upgrade().is_none() {
                break;
            }

            let block_result = block_stream.next().await;
            let block = match block_result {
                Ok(block) => block,
                Err(e) => {
                    log::warn!(
                        "[ContractFactoryCache] Could not retrieve next block: {:?}",
                        e
                    );
                    tokio::time::sleep(Duration::from_millis(DELAY_ON_TON_FAILURE)).await;
                    continue;
                }
            };

            loop {
                if let Some(inner) = weak_inner.upgrade() {
                    let process_result = inner.process_next_block(&block).await;
                    match process_result {
                        Ok(_) => break,
                        Err(e) => {
                            log::warn!(
                                "[ContractFactoryCache] Error processing block {}: {:?}",
                                block.master_shard.seqno,
                                e
                            );
                            tokio::time::sleep(Duration::from_millis(DELAY_ON_TON_FAILURE)).await;
                        }
                    }
                }
            }
        }

        log::info!("[ContractFactoryCache] Exiting run loop");
    }
}

struct Inner {
    client: TonClient,
    contract_state_cache: ContractStateCache,
    tx_id_cache: TxIdCache,
    account_state_cache: AccountStateCache,
}

impl Inner {
    async fn process_next_block(&self, block: &BlockStreamItem) -> Result<(), TonContractError> {
        log::trace!(
            "[ContractFactoryCache] Processing block: {}",
            block.master_shard.seqno
        );

        let mut all_shards = block.shards.clone();
        all_shards.push(block.master_shard.clone());

        let tx_ids: Vec<_> = self
            .client
            .get_shards_tx_ids(all_shards.as_slice())
            .await?
            .into_iter()
            .map(|(_, vec)| vec)
            .flatten()
            .collect();

        let mut contract_latest_tx_id: HashMap<TonAddress, InternalTransactionId> = HashMap::new();
        for tx_id in tx_ids.into_iter() {
            let id = tx_id.internal_transaction_id;

            if let Some(existing_item) = contract_latest_tx_id.get_mut(&tx_id.address) {
                if id.lt > existing_item.lt {
                    *existing_item = id;
                }
            } else {
                contract_latest_tx_id.insert(tx_id.address, id);
            }
        }

        let futures = contract_latest_tx_id
            .into_iter()
            .map(|(address, tx_id)| self.update_cache_entry(address, tx_id));
        join_all(futures).await;

        Ok(())
    }

    async fn update_cache_entry(&self, address: TonAddress, tx_id: InternalTransactionId) {
        self.tx_id_cache.insert(address.clone(), tx_id).await;
        join!(
            self.contract_state_cache.remove(&address),
            self.account_state_cache.remove(&address)
        );
    }
}

// There's a bug in tonlib 2023.6 that prevents smc.loadByTransaction & raw.getAccountStateByTransaction
// to work for ston.fi LpAccount after providing liquidity both tokens in a single transaction.
// This bug is fixed in tonlib 2023.11.
//
// We need this trait to detect & work around these situations before we can upgrade to the
// version of tonlib without this bug.
trait TransactionHashMismatch {
    fn is_transaction_hash_mismatch(&self) -> bool;
}

impl TransactionHashMismatch for TonClientError {
    fn is_transaction_hash_mismatch(&self) -> bool {
        match self {
            TonClientError::TonlibError { message, .. } => message == "transaction hash mismatch",
            _ => false,
        }
    }
}

impl TransactionHashMismatch for TonContractError {
    fn is_transaction_hash_mismatch(&self) -> bool {
        match self {
            TonContractError::ClientError(e) => e.is_transaction_hash_mismatch(),
            _ => false,
        }
    }
}
