use std::collections::HashSet;
use std::time::Duration;

use tokio::time;

use crate::client::{TonClient, TonClientError, TonClientInterface};
use crate::tl::{BlockId, BlockIdExt, BlocksShards};

#[derive(Debug, Clone)]
pub struct BlockStreamItem {
    pub master_shard: BlockIdExt,
    pub shards: Vec<BlockIdExt>,
}

/// Allows to sequentially retrieve all shards in all workchains.
///
/// The result of `next` call is the height of next masterchain block together with
/// all shards in all workchains that were finalized in corresponding masterchain block.
pub struct BlockStream {
    client: TonClient,
    next_seqno: i32,
    known_master_seqno: i32,
    prev_block_set: HashSet<BlockId>,
}

impl BlockStream {
    pub fn new(client: &TonClient, from_seqno: i32) -> BlockStream {
        BlockStream {
            client: client.clone(),
            next_seqno: from_seqno,
            known_master_seqno: 0,
            prev_block_set: Default::default(),
        }
    }

    /// Retrieves the next masterchain block together with all shards finalized in this block
    ///
    /// If the next block is not yet available, the returned future resolves when it's added to masterchain.
    pub async fn next(&mut self) -> Result<BlockStreamItem, TonClientError> {
        if self.prev_block_set.is_empty() {
            let (prev_block_shards, _) = self.get_master_block_shards(self.next_seqno - 1).await?;
            for shard in prev_block_shards.shards {
                self.prev_block_set.insert(shard.to_block_id());
            }
        };
        if self.known_master_seqno < self.next_seqno {
            loop {
                let masterchain_info = self.client.get_masterchain_info().await?;
                self.known_master_seqno = masterchain_info.last.seqno;
                if masterchain_info.last.seqno < self.next_seqno {
                    time::sleep(Duration::from_millis(100)).await;
                } else {
                    break;
                }
            }
        }
        let (block_shards, master_block) = self.get_master_block_shards(self.next_seqno).await?;
        let mut result_shards: HashSet<BlockIdExt> = Default::default();
        let mut unprocessed_shards: Vec<BlockIdExt> = Default::default();
        unprocessed_shards.extend(block_shards.shards.clone());
        while let Some(curr_shard) = unprocessed_shards.pop() {
            if self.prev_block_set.contains(&curr_shard.to_block_id()) {
                continue;
            }
            if result_shards.contains(&curr_shard) {
                continue;
            }
            result_shards.insert(curr_shard.clone());
            let curr_shard_ids = self.client.get_block_header(&curr_shard).await?;
            unprocessed_shards.extend(curr_shard_ids.prev_blocks);
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

    async fn get_master_block_shards(
        &self,
        seqno: i32,
    ) -> Result<(BlocksShards, BlockIdExt), TonClientError> {
        let master_block = BlockId {
            workchain: -1,
            shard: i64::MIN,
            seqno,
        };
        let master_block_ext = self.client.lookup_block(1, &master_block, 0, 0).await?;
        Ok((
            self.client.get_block_shards(&master_block_ext).await?,
            master_block_ext,
        ))
    }
}
