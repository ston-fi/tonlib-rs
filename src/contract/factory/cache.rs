use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;

use futures::future::join_all;
use futures::join;
use moka::future::Cache;

use crate::address::TonAddress;
use crate::client::{
    BlockStream, BlockStreamItem, TonBlockFunctions, TonClient, TonClientInterface,
};
use crate::contract::{LoadedSmcState, TonContractError};
use crate::tl::{InternalTransactionId, RawFullAccountState};

type TxIdCache = Cache<TonAddress, Arc<InternalTransactionId>>;
type AccountStateCache = Cache<TonAddress, Arc<RawFullAccountState>>;

const DELAY_ON_TON_FAILURE: u64 = 100;

#[derive(Clone)]
pub struct ContractFactoryCache {
    inner: Arc<Inner>,
}

impl ContractFactoryCache {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        client: &TonClient,
        account_state_cache_capacity: u64,
        account_state_cache_time_to_live: Duration,
        txid_cache_capacity: u64,
        txid_state_cache_time_to_live: Duration,
        presync_blocks: i32,
    ) -> Result<ContractFactoryCache, TonContractError> {
        let inner = Inner {
            client: client.clone(),

            account_state_cache: Cache::builder()
                .max_capacity(account_state_cache_capacity)
                .time_to_live(account_state_cache_time_to_live)
                .build(),
            tx_id_cache: Cache::builder()
                .max_capacity(txid_cache_capacity)
                .time_to_live(txid_state_cache_time_to_live)
                .build(),
            presync_blocks,
            account_state_cache_counters: ContractFactoryCacheCounters::default(),

            tx_id_cache_counters: ContractFactoryCacheCounters::default(),
        };

        let arc_inner = Arc::new(inner);
        let weak_inner = Arc::downgrade(&arc_inner);

        tokio::task::spawn(async move { Self::run_loop(weak_inner).await });

        let cache = ContractFactoryCache { inner: arc_inner };
        Ok(cache)
    }

    pub async fn get_smc_state_by_transaction(
        &self,
        address: &TonAddress,
        transaction_id: &InternalTransactionId,
    ) -> Result<Arc<LoadedSmcState>, TonContractError> {
        let loaded_state = self
            .inner
            .client
            .smc_load_by_transaction(address, transaction_id)
            .await?;

        Ok(Arc::new(loaded_state))
    }

    pub async fn get_account_state(
        &self,
        address: &TonAddress,
    ) -> Result<Arc<RawFullAccountState>, TonContractError> {
        self.inner
            .account_state_cache_counters
            .hits
            .fetch_add(1, Ordering::Relaxed);
        let state_result = self
            .inner
            .account_state_cache
            .try_get_with_by_ref(address, self.load_account_state(address))
            .await;

        match state_result {
            Ok(state) => Ok(state),
            Err(e) => Err(TonContractError::CacheError(e.clone())),
        }
    }

    async fn load_account_state(
        &self,
        address: &TonAddress,
    ) -> Result<Arc<RawFullAccountState>, TonContractError> {
        self.inner
            .account_state_cache_counters
            .misses
            .fetch_add(1, Ordering::Relaxed);
        self.inner
            .account_state_cache_counters
            .hits
            .fetch_sub(1, Ordering::Relaxed);

        let client = &self.inner.client;
        let tx_id_cache = &self.inner.tx_id_cache;
        let maybe_tx_id = tx_id_cache.get(address).await;
        let state = if let Some(tx_id) = maybe_tx_id {
            client
                .get_raw_account_state_by_transaction(address, &tx_id)
                .await?
        } else {
            client.get_raw_account_state(address).await?
        };
        Ok(Arc::new(state))
    }

    async fn run_loop(weak_inner: Weak<Inner>) {
        let mut block_stream = loop {
            if let Some(inner) = weak_inner.upgrade() {
                let client = &inner.client;
                let masterchain_info_result = client.get_masterchain_info().await;
                match masterchain_info_result {
                    Ok((_, info)) => {
                        let first_block_seqno = info.last.seqno - inner.presync_blocks;
                        let block_stream = BlockStream::new(client, first_block_seqno);
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

    pub fn get_cache_stats(&self) -> ContractFactoryCacheStats {
        ContractFactoryCacheStats {
            tx_id_cache_hits: self.inner.tx_id_cache_counters.hits.load(Ordering::Relaxed),
            tx_id_cache_misses: self
                .inner
                .tx_id_cache_counters
                .misses
                .load(Ordering::Relaxed),
            tx_id_cache_entry_count: self.inner.tx_id_cache.entry_count(),
            account_state_cache_hits: self
                .inner
                .account_state_cache_counters
                .hits
                .load(Ordering::Relaxed),
            account_state_cace_misses: self
                .inner
                .account_state_cache_counters
                .misses
                .load(Ordering::Relaxed),
            account_state_cache_entry_count: self.inner.account_state_cache.entry_count(),
        }
    }
}

struct Inner {
    client: TonClient,
    tx_id_cache: TxIdCache,
    account_state_cache: AccountStateCache,
    presync_blocks: i32,
    tx_id_cache_counters: ContractFactoryCacheCounters,
    account_state_cache_counters: ContractFactoryCacheCounters,
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
            .flat_map(|(_, vec)| vec)
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
        self.tx_id_cache
            .insert(address.clone(), Arc::new(tx_id.clone()))
            .await;
        join!(self.account_state_cache.invalidate(&address),);
    }
}

#[derive(Default)]
pub struct ContractFactoryCacheStats {
    pub tx_id_cache_hits: u64,
    pub tx_id_cache_misses: u64,
    pub tx_id_cache_entry_count: u64,
    pub account_state_cache_hits: u64,
    pub account_state_cace_misses: u64,
    pub account_state_cache_entry_count: u64,
}

#[derive(Default)]
struct ContractFactoryCacheCounters {
    hits: AtomicU64,
    misses: AtomicU64,
}
