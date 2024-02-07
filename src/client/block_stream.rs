use std::collections::HashSet;
use std::time::Duration;

use futures::future::try_join_all;
use tokio::time;

use crate::client::{TonClient, TonClientError, TonClientInterface, TonConnection};
use crate::tl::{BlockId, BlockIdExt, BlocksHeader, BlocksShards};

#[derive(Debug, Clone)]
pub struct BlockStreamItem {
    pub master_shard: BlockIdExt,
    pub shards: Vec<BlockIdExt>,
}

/// Allows to sequentially retrieve all shards in all workchains.
///
/// The result of `next` call is the height of next masterchain block together with
/// all shards in all workchains that were finalized in corresponding masterchain block.
///
pub struct BlockStream {
    client: TonClient,
    next_seqno: i32,
    prev_block_set: HashSet<BlockId>,
}

impl BlockStream {
    pub fn new(client: &TonClient, from_seqno: i32) -> BlockStream {
        BlockStream {
            client: client.clone(),
            next_seqno: from_seqno,
            prev_block_set: Default::default(),
        }
    }

    /// Retrieves the next masterchain block together with all shards finalized in this block
    ///
    /// If the next block is not yet available, the returned future resolves when it's added to masterchain.
    pub async fn next(&mut self) -> Result<BlockStreamItem, TonClientError> {
        if self.prev_block_set.is_empty() {
            let (prev_block_shards, _) =
                Self::get_master_block_shards(&self.client, self.next_seqno - 1).await?;
            for shard in prev_block_shards.shards {
                self.prev_block_set.insert(shard.to_block_id());
            }
        };
        let connection = loop {
            let (conn, masterchain_info) = self.client.get_masterchain_info().await?;
            if masterchain_info.last.seqno < self.next_seqno {
                time::sleep(Duration::from_millis(100)).await;
            } else {
                break conn;
            }
        };
        let (block_shards, master_block) =
            Self::get_master_block_shards(&connection, self.next_seqno).await?;
        let mut result_shards: HashSet<BlockIdExt> = Default::default();
        let mut unprocessed_shards: Vec<BlockIdExt> = Default::default();
        unprocessed_shards.extend(block_shards.shards.clone());
        while !unprocessed_shards.is_empty() {
            let mut shards_to_process: HashSet<BlockIdExt> = Default::default();
            for s in unprocessed_shards.into_iter() {
                if self.prev_block_set.contains(&s.to_block_id()) {
                    continue;
                }
                if result_shards.contains(&s) {
                    continue;
                }
                result_shards.insert(s.clone());
                shards_to_process.insert(s);
            }
            unprocessed_shards = Default::default();
            let headers = self
                .get_block_headers(&connection, &shards_to_process)
                .await?;
            for h in headers {
                if let Some(prev_blocks) = h.prev_blocks {
                    unprocessed_shards.extend(prev_blocks)
                }
            }
        }

        self.next_seqno += 1;
        let new_prev_seq_shards = block_shards.shards;
        self.prev_block_set = new_prev_seq_shards
            .into_iter()
            .map(|shard| shard.to_block_id())
            .collect();
        Ok(BlockStreamItem {
            shards: result_shards.into_iter().collect(),
            master_shard: master_block,
        })
    }

    async fn get_block_headers(
        &self,
        conn: &TonConnection,
        shards: &HashSet<BlockIdExt>,
    ) -> Result<Vec<BlocksHeader>, TonClientError> {
        let futures: Vec<_> = shards
            .iter()
            .map(|id| self.retrying_get_block_header(conn, id))
            .collect();
        let r = try_join_all(futures).await?;
        Ok(r)
    }

    async fn retrying_get_block_header(
        &self,
        conn: &TonConnection,
        block_id: &BlockIdExt,
    ) -> Result<BlocksHeader, TonClientError> {
        let r = conn.get_block_header(block_id).await;
        // Fallback to random connection on client
        match r {
            Ok(bh) => Ok(bh),
            Err(_) => self.client.get_block_header(block_id).await,
        }
    }

    async fn get_master_block_shards<C: TonClientInterface>(
        conn: &C,
        seqno: i32,
    ) -> Result<(BlocksShards, BlockIdExt), TonClientError> {
        let master_block = BlockId {
            workchain: -1,
            shard: i64::MIN,
            seqno,
        };
        let master_block_ext = conn.lookup_block(1, &master_block, 0, 0).await?;
        Ok((
            conn.get_block_shards(&master_block_ext).await?,
            master_block_ext,
        ))
    }
}
