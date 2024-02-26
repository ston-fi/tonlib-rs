#![cfg(feature = "interactive")]

use tonlib::client::{BlockStream, TonClientInterface};

mod common;

#[tokio::test]
pub async fn block_listener_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await?;
    let (_, mc_info) = client.get_masterchain_info().await?;
    println!("{:?}", mc_info);
    let seqno = mc_info.last.seqno - 20;
    let mut listener = BlockStream::new(&client, seqno);
    for _ in 0..10 {
        let block = listener.next().await?;
        println!(
            "seqno {}: master shard {:?}: shards: {:?}",
            block.master_shard.seqno,
            block.master_shard.to_block_id(),
            block
                .shards
                .iter()
                .map(|s| (s.workchain, s.shard, s.seqno))
                .collect::<Vec<_>>()
        );
    }
    Ok(())
}

#[tokio::test]
pub async fn block_listener_get_block_header() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_testnet_client().await?;
    let seqno = client.get_masterchain_info().await?.1.last;
    let headers = client.get_block_header(&seqno).await?;
    println!("{:?}", headers);
    Ok(())
}
