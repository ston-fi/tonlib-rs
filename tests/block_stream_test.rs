use tonlib::client::{BlockStream, TonBlockFunctions, TonClient, TonClientInterface};

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

const CONFIG_N03: &str = include_str!("../resources/config/stonfi-n03.json");

#[tokio::test]
#[ignore]
async fn test_connection_hang() -> anyhow::Result<()> {
    common::init_logging();
    let client = TonClient::builder()
        .with_config(CONFIG_N03)
        .with_pool_size(1)
        .build()
        .await?;
    let seqno = client.get_masterchain_info().await?.1.last.seqno;
    let mut block_stream = BlockStream::new(&client, seqno);
    let mut current = seqno;
    let until = seqno + 10;
    while current < until {
        let item = block_stream.next().await?;
        log::info!("Received item: {}", item.master_shard.seqno);
        current = item.master_shard.seqno;
        for shard_id in item.shards.iter() {
            let txs = client.get_shard_tx_ids(shard_id).await?;
            for tx in txs {
                //let _ = client.smc_load_by_transaction(&tx.address, &tx.internal_transaction_id).await?;
                let r = client
                    .get_raw_account_state_by_transaction(&tx.address, &tx.internal_transaction_id)
                    .await;
                if let Err(e) = r {
                    log::warn!("Error retrieving state of {}: {:?}", &tx.address, e);
                }
            }
        }
    }
    Ok(())
}
