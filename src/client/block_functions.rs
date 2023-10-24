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
    pub internal_transaction_id: InternalTransactionId,
    pub raw_transaction: RawTransaction,
}

/// High-level functions for working with blocks & shards
#[async_trait]
pub trait TonBlockFunctions: TonClientInterface + Send + Sync {
    /// Returns the list of all transaction IDs in specified shard.
    async fn get_shard_tx_ids(
        &self,
        shard_ext: &BlockIdExt,
    ) -> Result<Vec<BlocksShortTxId>, TonClientError> {
        let mut after: BlocksAccountTransactionId = NULL_BLOCKS_ACCOUNT_TRANSACTION_ID.clone();
        let mut transactions: Vec<BlocksShortTxId> = Vec::new();
        loop {
            let mode = if after.lt == 0 { 7 } else { 128 + 7 };
            let txs: BlocksTransactions = self
                .get_block_transactions(&shard_ext, mode, 256, &after)
                .await?;
            if let Some(last) = txs.transactions.last() {
                after = BlocksAccountTransactionId {
                    account: last.account.clone(),
                    lt: last.lt,
                };
            }
            transactions.extend(txs.transactions);
            if !txs.incomplete {
                break;
            }
        }
        Ok(transactions)
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
                .map(|tx_id| load_raw_tx(self, shard_id.workchain, tx_id).boxed())
                .collect();
        let txs: Vec<TxData> = try_join_all(futures).await?;
        Ok(txs)
    }

    /// Returns all transactions from specified shards
    async fn get_shards_transactions(
        &self,
        shards: &Vec<BlockIdExt>,
    ) -> Result<Vec<(BlockIdExt, Vec<TxData>)>, TonClientError> {
        let f = shards.iter().map(|shard| {
            self.get_shard_transactions(shard)
                .map(move |txs_r| txs_r.map(|txs| (shard.clone(), txs)))
        });
        let txs: Vec<_> = try_join_all(f).await?;
        Ok(txs)
    }
}

impl<T> TonBlockFunctions for T where T: TonClientInterface + Send + Sync {}

async fn load_raw_tx<T: TonClientInterface + Send + Sync + ?Sized>(
    client: &T,
    workchain: i32,
    tx_id: &BlocksShortTxId,
) -> Result<TxData, TonClientError> {
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
    let tx_result = client.get_raw_transactions_v2(&addr, &id, 1, false).await?;
    let tx = if tx_result.transactions.len() == 1 {
        tx_result.transactions[0].clone()
    } else {
        return Err(TonClientError::InternalError {
            message: format!(
                "Expected 1 tx, got {}, query: {:?}/{:?}",
                tx_result.transactions.len(),
                addr,
                id
            ),
        });
    };
    Ok(TxData {
        address: addr,
        internal_transaction_id: id,
        raw_transaction: tx,
    })
}
