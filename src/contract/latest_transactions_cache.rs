use std::collections::LinkedList;
use std::ops::Sub;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::Mutex;

use crate::address::TonAddress;
use crate::client::TonClientError;
use crate::contract::{TonClientInterface, TonContractError, TonContractFactory};
use crate::tl::{InternalTransactionId, RawTransaction, NULL_TRANSACTION_ID};

pub struct LatestContractTransactionsCache {
    capacity: usize,
    contract_factory: TonContractFactory,
    address: TonAddress,

    soft_limit: bool,
    tx_age_limit: Option<Duration>,
    inner: Mutex<Inner>,
}

impl LatestContractTransactionsCache {
    pub fn new(
        contract_factory: &TonContractFactory,
        address: &TonAddress,
        capacity: usize,
        soft_limit: bool,
        tx_age_limit: Option<Duration>,
    ) -> LatestContractTransactionsCache {
        let inner = Mutex::new(Inner {
            transactions: LinkedList::new(),
        });
        LatestContractTransactionsCache {
            capacity,
            contract_factory: contract_factory.clone(),
            address: address.clone(),

            soft_limit,
            tx_age_limit,
            inner,
        }
    }

    pub async fn get(&self, limit: usize) -> Result<Vec<Arc<RawTransaction>>, TonContractError> {
        if limit > self.capacity {
            return Err(TonContractError::IllegalArgument(format!(
                "Transactions cache size requested ({}) must not exceed cache capacity ({})",
                limit, self.capacity
            )));
        }

        let target_sync_tx_id = self.get_latest_tx_id().await?;

        let mut inner = self.inner.lock().await;

        // check sync status
        if inner.is_not_synced_to_tx_id(&target_sync_tx_id) {
            inner
                .load_new_txs(
                    &self.contract_factory,
                    &self.address,
                    self.soft_limit,
                    self.capacity,
                    &target_sync_tx_id,
                    self.tx_age_limit,
                )
                .await?;
        }
        let r = inner.fill_txs(limit);
        Ok(r)
    }

    pub async fn get_all(&self) -> Result<Vec<Arc<RawTransaction>>, TonContractError> {
        self.get(self.capacity).await
    }

    async fn get_latest_tx_id(&self) -> Result<InternalTransactionId, TonContractError> {
        let state = self
            .contract_factory
            .get_latest_account_state(&self.address)
            .await?;
        let tx_id = state.last_transaction_id.clone();
        Ok(tx_id)
    }
}

struct Inner {
    transactions: LinkedList<Arc<RawTransaction>>,
}

impl Inner {
    async fn load_new_txs(
        &mut self,
        contract_factory: &TonContractFactory,
        address: &TonAddress,
        soft_limit: bool,
        capacity: usize,
        target_sync_tx: &InternalTransactionId,
        tx_age_limit: Option<Duration>,
    ) -> Result<(), TonContractError> {
        let synced_tx_id = self.get_latest_synced_tx_id();
        let mut loaded = Vec::new();
        let mut finished = false;
        let mut next_to_load = target_sync_tx.clone();
        let mut batch_size = 16;

        if next_to_load.lt <= synced_tx_id.lt {
            log::warn!(
                "next to load lt is less or equal to synced_tx_id.lt {:?},{:?}",
                next_to_load.lt,
                synced_tx_id.lt
            );
            return Ok(());
        }

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_e| TonContractError::InternalError("Time went backwards!".to_string()))?;
        let min_utime = tx_age_limit.map(|duration| current_time - duration);

        while !finished && next_to_load.lt != 0 && next_to_load.lt > synced_tx_id.lt {
            let maybe_txs = contract_factory
                .clone()
                .client()
                .get_raw_transactions_v2(address, &next_to_load, batch_size, false)
                .await;
            let txs = match maybe_txs {
                Ok(txs) => txs,
                Err(e) if soft_limit => match e {
                    TonClientError::TonlibError { code: 500, .. } => {
                        batch_size /= 2;
                        if batch_size == 0 {
                            break;
                        } else {
                            continue;
                        }
                    }
                    _ => break,
                },
                Err(e) => {
                    return Err(e.into());
                }
            };

            for tx in txs.transactions {
                if loaded.len() >= capacity || tx.transaction_id.lt <= synced_tx_id.lt {
                    finished = true;
                    break;
                } else if Inner::is_older_than(tx.utime, min_utime) {
                    finished = true;
                    log::trace!(
                        "Minimum loaded timestamp limit reached {:?} for transaction id {:?}",
                        tx_age_limit.map(|limit| current_time.sub(limit)),
                        tx.transaction_id.lt
                    );
                    break;
                }
                loaded.push(Arc::new(tx));
            }
            next_to_load = txs.previous_transaction_id.clone();
        }
        // Add loaded transactions
        if !loaded.is_empty() {
            log::trace!(
                "Adding {} new transactions for contract {}",
                loaded.len(),
                address
            );
        }
        let txs = &mut self.transactions;
        for tx in loaded.iter().rev() {
            txs.push_front(tx.clone());
        }

        // Remove outdated transactions
        if txs.len() > capacity {
            log::trace!(
                "Removing {} outdated transactions for contract {}",
                txs.len() - capacity,
                address
            );
        }
        while txs.len() > capacity {
            txs.pop_back();
        }
        log::trace!("Finished sync");

        Ok(())
    }

    fn is_older_than(utime: i64, min_utime: Option<Duration>) -> bool {
        if let Some(min) = min_utime {
            return Duration::from_secs(utime as u64) < min;
        }
        false
    }

    fn fill_txs(&self, limit: usize) -> Vec<Arc<RawTransaction>> {
        let mut res = Vec::with_capacity(limit);
        let txs = &self.transactions;
        for i in txs.iter().take(limit) {
            res.push(i.clone())
        }
        res
    }

    fn get_latest_synced_tx_id(&self) -> &InternalTransactionId {
        self.transactions
            .front()
            .map(|tx| &tx.transaction_id)
            .unwrap_or(&NULL_TRANSACTION_ID)
    }

    fn is_not_synced_to_tx_id(&self, target_sync_tx: &InternalTransactionId) -> bool {
        let latest_synced_tx = self.get_latest_synced_tx_id();
        latest_synced_tx != target_sync_tx
    }
}
