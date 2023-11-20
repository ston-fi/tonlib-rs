use async_trait::async_trait;
use futures::future::try_join_all;
use futures::FutureExt;
use std::future::Future;
use std::pin::Pin;

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
            tx_id
                .account
                .as_slice()
                .try_into()
                .map_err(|_| TonClientError::InternalError {
                    message: format!("Invalid BlocksShortTxId: {:?}", tx_id),
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
                .get_block_transactions(&shard_id, mode, 256, &after)
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
    ) -> Result<Vec<TxData>, TonClientError> {
        let tx_ids = self.get_shard_tx_ids(shard_id).await?;
        let futures: Vec<Pin<Box<dyn Future<Output = Result<TxData, TonClientError>> + Send>>> =
            tx_ids
                .iter()
                .map(|tx_id| load_raw_tx(self, tx_id).boxed())
                .collect();
        let txs: Vec<TxData> = try_join_all(futures).await?;
        Ok(txs)
    }

    /// Returns all transactions from specified shards
    async fn get_shards_transactions(
        &self,
        shards: &[BlockIdExt],
    ) -> Result<Vec<(BlockIdExt, Vec<TxData>)>, TonClientError> {
        let f = shards.iter().map(|shard| {
            self.get_shard_transactions(shard)
                .map(move |res| res.map(|txs| (shard.clone(), txs)))
        });
        let txs: Vec<_> = try_join_all(f).await?;
        Ok(txs)
    }
}

impl<T> TonBlockFunctions for T where T: TonClientInterface + Send + Sync {}

async fn load_raw_tx<T: TonClientInterface + Send + Sync + ?Sized>(
    client: &T,
    tx_id: &TxId,
) -> Result<TxData, TonClientError> {
    let tx_result = client
        .get_raw_transactions_v2(&tx_id.address, &tx_id.internal_transaction_id, 1, false)
        .await?;
    let tx = if tx_result.transactions.len() == 1 {
        tx_result.transactions[0].clone()
    } else {
        return Err(TonClientError::InternalError {
            message: format!(
                "Expected 1 tx, got {}, query: {:?}/{:?}",
                tx_result.transactions.len(),
                tx_id.address,
                tx_id.internal_transaction_id
            ),
        });
    };
    Ok(TxData {
        address: tx_id.address.clone(),
        raw_transaction: tx,
    })
}
