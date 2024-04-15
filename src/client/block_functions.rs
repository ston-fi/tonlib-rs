use async_trait::async_trait;
use futures::future::try_join_all;
use futures::FutureExt;

use crate::address::TonAddress;
use crate::client::{TonClientError, TonClientInterface};
use crate::tl::{
    BlockIdExt, BlocksAccountTransactionId, BlocksShortTxId, BlocksTransactions,
    InternalTransactionId, RawTransaction, NULL_BLOCKS_ACCOUNT_TRANSACTION_ID,
};

#[derive(Debug, Clone)]
pub struct TxData {
    pub address: TonAddress,
    pub raw_transaction: RawTransaction,
}

#[derive(Debug, Clone)]
pub struct TxId {
    pub address: TonAddress,
    pub internal_transaction_id: InternalTransactionId,
}

impl TxId {
    pub fn new(workchain: i32, tx_id: &BlocksShortTxId) -> Result<TxId, TonClientError> {
        let addr = TonAddress::new(
            workchain,
            tx_id.account.as_slice().try_into().map_err(|_| {
                TonClientError::InternalError(format!("Invalid BlocksShortTxId: {:?}", tx_id))
            })?,
        );
        let id = InternalTransactionId {
            lt: tx_id.lt,
            hash: tx_id.hash.clone(),
        };
        Ok(TxId {
            address: addr,
            internal_transaction_id: id,
        })
    }
}

/// High-level functions for working with blocks & shards
#[async_trait]
pub trait TonBlockFunctions: TonClientInterface + Send + Sync {
    /// Returns the list of all transaction IDs in specified shard.
    async fn get_shard_tx_ids(&self, shard_id: &BlockIdExt) -> Result<Vec<TxId>, TonClientError> {
        let mut after: BlocksAccountTransactionId = NULL_BLOCKS_ACCOUNT_TRANSACTION_ID.clone();
        let mut transactions: Vec<TxId> = Vec::new();
        loop {
            let mode = if after.lt == 0 { 7 } else { 128 + 7 };
            let txs: BlocksTransactions = self
                .get_block_transactions(shard_id, mode, 256, &after)
                .await?;
            if let Some(last) = txs.transactions.last() {
                after = BlocksAccountTransactionId {
                    account: last.account.clone(),
                    lt: last.lt,
                };
            }
            transactions.reserve(txs.transactions.len());
            for tx in txs.transactions {
                transactions.push(TxId::new(shard_id.workchain, &tx)?)
            }
            if !txs.incomplete {
                break;
            }
        }
        Ok(transactions)
    }

    async fn get_shards_tx_ids(
        &self,
        shards: &[BlockIdExt],
    ) -> Result<Vec<(BlockIdExt, Vec<TxId>)>, TonClientError> {
        let f = shards.iter().map(|shard| {
            self.get_shard_tx_ids(shard)
                .map(move |r| r.map(|tx_ids| (shard.clone(), tx_ids)))
        });
        let txs: Vec<_> = try_join_all(f).await?;
        Ok(txs)
    }

    /// Returns all transactions from specified shard
    async fn get_shard_transactions(
        &self,
        shard_id: &BlockIdExt,
    ) -> Result<Vec<RawTransaction>, TonClientError> {
        let mut after: BlocksAccountTransactionId = NULL_BLOCKS_ACCOUNT_TRANSACTION_ID.clone();
        let mut raw_txs: Vec<RawTransaction> = Vec::new();
        loop {
            let mode = if after.lt == 0 { 7 } else { 128 + 7 };
            let txs = self
                .get_block_transactions_ext(shard_id, mode, 256, &after)
                .await?;
            if let Some(last) = txs.transactions.last() {
                let account = last.address.account_address.clone().into();
                let lt = last.transaction_id.lt;
                after = BlocksAccountTransactionId { account, lt };
            }
            raw_txs.reserve(txs.transactions.len());
            raw_txs.extend(txs.transactions);
            if !txs.incomplete {
                break;
            }
        }
        Ok(raw_txs)
    }
    /// Returns all transactions from specified shards
    async fn get_shards_transactions(
        &self,
        shards: &[BlockIdExt],
    ) -> Result<Vec<(BlockIdExt, Vec<RawTransaction>)>, TonClientError> {
        let f = shards.iter().map(|shard| {
            self.get_shard_transactions(shard)
                .map(move |res| res.map(|txs| (shard.clone(), txs)))
        });
        let txs: Vec<_> = try_join_all(f).await?;
        Ok(txs)
    }
}

impl<T> TonBlockFunctions for T where T: TonClientInterface + Send + Sync {}
