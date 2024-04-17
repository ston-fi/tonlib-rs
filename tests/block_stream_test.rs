use tonlib::client::{
    BlockStream, TonBlockFunctions, TonClientInterface, TonConnection, TonConnectionParams,
    LOGGING_CONNECTION_CALLBACK,
};
use tonlib::tl::InternalTransactionId;

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

#[tokio::test]
#[ignore]
async fn test_connection_hang() -> anyhow::Result<()> {
    common::init_logging();
    let params = TonConnectionParams::default();
    let client = TonConnection::connect(&params, LOGGING_CONNECTION_CALLBACK.clone()).await?;
    let seqno = client.get_masterchain_info().await?.1.last.seqno;
    let mut block_stream = BlockStream::new(&client, seqno);
    let mut current = seqno;
    let until = seqno + 10;
    const MAX_STATES_PER_BLOCK: u32 = 30;
    while current < until {
        let item = block_stream.next().await?;
        let mut states_processed = 0;
        log::info!("Received item: {}", item.master_shard.seqno);
        current = item.master_shard.seqno;
        for shard_id in item.shards.iter() {
            let txs = client.get_shard_tx_ids(shard_id).await?;
            for tx in txs {
                if states_processed < MAX_STATES_PER_BLOCK {
                    // let r = client
                    //     .smc_load_by_transaction(&tx.address, &tx.internal_transaction_id)
                    //     .await;
                    log::info!(
                        "Requesting {} {}:{}",
                        tx.address,
                        tx.internal_transaction_id.lt,
                        hex::encode(tx.internal_transaction_id.hash.as_slice())
                    );
                    let r = client
                        .get_raw_account_state_by_transaction(
                            &tx.address,
                            &tx.internal_transaction_id,
                        )
                        .await;
                    if let Err(e) = r {
                        log::error!(
                            "Error retrieving state of {}:{} {:?}",
                            &tx.address,
                            tx.internal_transaction_id,
                            e
                        );
                    }
                    states_processed += 1;
                }
            }
        }
    }
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_connection_hang_tx() -> anyhow::Result<()> {
    common::init_logging();
    let params = TonConnectionParams::default();
    let client = TonConnection::connect(&params, LOGGING_CONNECTION_CALLBACK.clone()).await?;
    let addr = "EQCqNjAPkigLdS5gxHiHitWuzF3ZN-gX7MlX4Qfy2cGS3FWx".parse()?;
    let tx_id = InternalTransactionId {
        lt: 45790671000001,
        hash: hex::decode("cb4a301e3aa15ca8eaad9c999d380fa7f6715976c7cb456e5a93fd8ebd3fb7f2")?,
    };
    client
        .get_raw_account_state_by_transaction(&addr, &tx_id)
        .await?;
    // client.smc_load_by_transaction(&addr, &tx_id).await?;
    Ok(())
}
