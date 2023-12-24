use std::collections::HashMap;
use std::ops::Deref;
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
        let client_clone = client.clone();

        tokio::task::spawn(async move { Self::run_loop(weak_inner, client_clone).await });

        let cache = ContractFactoryCache { inner: arc_inner };
        Ok(cache)
    }

    pub(crate) async fn get_contract_state(
        &self,
        client: &TonClient,
        address: &TonAddress,
    ) -> Result<TonContractState, TonContractError> {
        let state_result = self
            .inner
            .contract_state_cache
            .try_get_with(
                address.clone(),
                Self::load_contract_state(client, &self.inner.tx_id_cache, address),
            )
            .await;
        match state_result {
            Ok(state) => Ok(state),
            Err(e) => self.try_recover_hash_mismatch(client, address, &e).await,
        }
    }

    // There's a bug in tonlib that prevents smc.loadByTransaction to work for ston.fi LpAccount
    // after providing liquidity in a single operation.
    // In this situation we fall back to normal smc.load without caching
    async fn try_recover_hash_mismatch(
        &self,
        client: &TonClient,
        address: &TonAddress,
        e: &Arc<TonContractError>,
    ) -> Result<TonContractState, TonContractError> {
        match e.deref() {
            TonContractError::ClientError(TonClientError::TonlibError { message, .. }) => {
                if message == "transaction hash mismatch" {
                    return TonContractState::load(client, address).await;
                }
            }
            _ => {}
        }
        return Err(CacheError(e.clone()));
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

    pub async fn get_account_state(
        &self,
        client: &TonClient,
        account_address: &TonAddress,
    ) -> Result<RawFullAccountState, TonContractError> {
        let maybe_state = self
            .inner
            .account_state_cache
            .try_get_with(
                account_address.clone(),
                Self::load_account_state(client, account_address),
            )
            .await;
        match maybe_state {
            Ok(state) => Ok(state),
            Err(e) => Err(TonContractError::InternalError {
                message: format!("{:?}", e),
            }),
        }
    }

    async fn load_account_state(
        client: &TonClient,
        address: &TonAddress,
    ) -> Result<RawFullAccountState, TonContractError> {
        let state = client.get_raw_account_state(address).await?;
        Ok(state)
    }

    async fn run_loop(weak_inner: Weak<Inner>, client: TonClient) {
        let first_block_seqno = loop {
            let masterchain_info_result = client.get_masterchain_info().await;
            match masterchain_info_result {
                Ok((_, info)) => break info.last.seqno,
                Err(e) => {
                    log::warn!(
                        "[ContractFactoryCache] Could not retrieve current block: {}",
                        e
                    );
                    tokio::time::sleep(Duration::from_millis(DELAY_ON_TON_FAILURE)).await;
                }
            }
        };

        let mut block_stream = BlockStream::new(&client, first_block_seqno);

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
                        "[ContractFactoryCache] Could not retrieve next block: {}",
                        e
                    );
                    tokio::time::sleep(Duration::from_millis(DELAY_ON_TON_FAILURE)).await;
                    continue;
                }
            };

            loop {
                if let Some(inner) = weak_inner.upgrade() {
                    let process_result = inner.process_next_block(&client, &block).await;
                    match process_result {
                        Ok(_) => break,
                        Err(e) => {
                            log::warn!(
                                "[ContractFactoryCache] Error processing block {}: {}",
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
    contract_state_cache: ContractStateCache,
    tx_id_cache: TxIdCache,
    account_state_cache: AccountStateCache,
}

impl Inner {
    async fn process_next_block(
        &self,
        client: &TonClient,
        block: &BlockStreamItem,
    ) -> Result<(), TonContractError> {
        log::trace!(
            "[ContractFactoryCache] Processing block: {}",
            block.master_shard.seqno
        );

        let mut all_shards = block.shards.clone();
        all_shards.push(block.master_shard.clone());

        let tx_ids: Vec<_> = client
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
