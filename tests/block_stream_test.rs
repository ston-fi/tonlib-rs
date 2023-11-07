use tonlib::client::{BlockStream, TonClientInterface};

mod common;

#[tokio::test]
pub async fn block_listener_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let seqno = client.get_masterchain_info().await?.last.seqno - 20;
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
    let client = common::new_test_client().await?;
    let seqno = client.get_masterchain_info().await?.last;
    let headers = client.get_block_header(&seqno).await?;
    println!("{:?}", headers);
    Ok(())
}
