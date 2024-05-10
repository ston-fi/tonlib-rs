use std::collections::LinkedList;
use std::ops::DerefMut;
use std::sync::Arc;

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
    inner: Mutex<Inner>,
}

struct Inner {
    transactions: LinkedList<Arc<RawTransaction>>,
}

impl LatestContractTransactionsCache {
    pub fn new(
        contract_factory: &TonContractFactory,
        contract_address: &TonAddress,
        capacity: usize,
        soft_limit: bool,
    ) -> LatestContractTransactionsCache {
        LatestContractTransactionsCache {
            capacity,
            contract_factory: contract_factory.clone(),
            address: contract_address.clone(),
            soft_limit,
            inner: Mutex::new(Inner {
                transactions: LinkedList::new(),
            }),
        }
    }

    /// Returns up to `limit` last transactions.
    ///
    /// Returned transactions are sorted from latest to earliest.
    pub async fn get(&self, limit: usize) -> Result<Vec<Arc<RawTransaction>>, TonContractError> {
        if limit > self.capacity {
            return Err(TonContractError::IllegalArgument(format!(
                "Transactions cache size requested ({}) must not exceed cache capacity ({})",
                limit, self.capacity
            )));
        }
        let mut lock = self.inner.lock().await;
        self.sync(lock.deref_mut()).await?;

        let mut res = Vec::with_capacity(limit);
        for i in lock.transactions.iter().take(limit) {
            res.push(i.clone())
        }
        Ok(res)
    }

    /// Returns up to `capacity` last transactions.
    ///
    /// Returned transactions are sorted from latest to earliest.
    pub async fn get_all(&self) -> Result<Vec<Arc<RawTransaction>>, TonContractError> {
        self.get(self.capacity).await
    }

    async fn sync(&self, inner: &mut Inner) -> Result<(), TonContractError> {
        // Find out what to sync
        let state = self
            .contract_factory
            .get_latest_account_state(&self.address)
            .await?;
        let last_tx_id = &state.last_transaction_id;

        let synced_tx_id: &InternalTransactionId = inner
            .transactions
            .front()
            .map(|tx| &tx.transaction_id)
            .unwrap_or(&NULL_TRANSACTION_ID);

        // Load neccessary data
        let mut loaded: Vec<Arc<RawTransaction>> = Vec::new();
        let mut finished = false;
        let mut next_to_load: InternalTransactionId = last_tx_id.clone();
        let mut batch_size: usize = 16;
        while !finished && next_to_load.lt != 0 && next_to_load.lt > synced_tx_id.lt {
            let maybe_txs = self
                .contract_factory
                .client()
                .get_raw_transactions_v2(&self.address, &next_to_load, batch_size, false)
                .await;
            let txs = match maybe_txs {
                Ok(txs) => txs,
                Err(e) if self.soft_limit => match e {
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
                if loaded.len() >= self.capacity || tx.transaction_id.lt <= synced_tx_id.lt {
                    finished = true;
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
                self.address
            );
        }
        for tx in loaded.iter().rev() {
            inner.transactions.push_front(tx.clone());
        }

        // Remove outdated transactions
        if inner.transactions.len() > self.capacity {
            log::trace!(
                "Removing {} outdated transactions for contract {}",
                inner.transactions.len() - self.capacity,
                self.address
            );
        }
        while inner.transactions.len() > self.capacity {
            inner.transactions.pop_back();
        }

        Ok(())
    }
}
