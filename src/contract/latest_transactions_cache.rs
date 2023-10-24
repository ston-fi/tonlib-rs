use std::collections::LinkedList;
use std::ops::DerefMut;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::address::TonAddress;
use crate::client::TonClient;
use crate::contract::{TonContract, TonContractInterface, TransactionError};
use crate::tl::{InternalTransactionId, RawTransaction, NULL_TRANSACTION_ID};

pub struct LatestContractTransactionsCache {
    capacity: usize,
    contract: TonContract,
    inner: Mutex<Inner>,
}

struct Inner {
    transactions: LinkedList<Arc<RawTransaction>>,
}

impl LatestContractTransactionsCache {
    pub fn new(
        client: &TonClient,
        contract_address: &TonAddress,
        capacity: usize,
    ) -> LatestContractTransactionsCache {
        LatestContractTransactionsCache {
            capacity,
            contract: TonContract::new(client, &contract_address.clone()),
            inner: Mutex::new(Inner {
                transactions: LinkedList::new(),
            }),
        }
    }

    /// Returns up to `limit` last transactions.
    ///
    /// Returned transactions are sorted from latest to earliest.
    pub async fn get(&self, limit: usize) -> Result<Vec<Arc<RawTransaction>>, TransactionError> {
        if limit > self.capacity {
            return Err(TransactionError::LimitExceeded {
                limit: limit,
                capacity: self.capacity,
            });
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
    pub async fn get_all(&self) -> Result<Vec<Arc<RawTransaction>>, TransactionError> {
        self.get(self.capacity).await
    }

    async fn sync(&self, inner: &mut Inner) -> Result<(), TransactionError> {
        // Find out what to sync
        let state = self.contract.get_account_state().await?;
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
        while !finished && next_to_load.lt != 0 && next_to_load.lt > synced_tx_id.lt {
            let txs = self
                .contract
                .get_raw_transactions(&next_to_load, 16)
                .await?;

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
        if loaded.len() > 0 {
            log::trace!(
                "Adding {} new transactions for contract {}",
                loaded.len(),
                self.contract.address()
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
                self.contract.address()
            );
        }
        while inner.transactions.len() > self.capacity {
            inner.transactions.pop_back();
        }

        Ok(())
    }
}
