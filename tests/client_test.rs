use std::thread;
use std::time::Duration;

use tokio;
use tokio::time::timeout;

use tonlib::address::TonAddress;
use tonlib::client::TonFunctions;
use tonlib::tl::types::{
    AccountState, BlockId, BlocksMasterchainInfo, BlocksShards, BlocksTransactions,
    InternalTransactionId, SmcMethodId, NULL_BLOCKS_ACCOUNT_TRANSACTION_ID,
};

mod common;

#[tokio::test]
async fn client_get_account_state_of_inactive_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let r = client
        .get_account_state("EQDOUwuz-6lH-IL-hqSHQSrFhoNjTNjKp04Wb5n2nkctCJTH")
        .await;
    println!("{:?}", r);
    assert!(r.is_ok());
    match r.unwrap().account_state {
        AccountState::Uninited { .. } => {}
        _ => {
            panic!("Expected UnInited state")
        }
    }
    Ok(())
}

#[tokio::test]
async fn client_get_raw_account_state_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let r = client
        .get_raw_account_state("EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR")
        .await;
    println!("{:?}", r);
    assert!(r.is_ok());
    Ok(())
}

#[tokio::test]
async fn client_get_raw_transactions_works() -> anyhow::Result<()> {
    common::init_logging();
    let address = "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR";
    let client = common::new_test_client().await?;
    let state = client.get_raw_account_state(address).await.unwrap();
    let r = client
        .get_raw_transactions(address, &state.last_transaction_id)
        .await;
    println!("{:?}", r);
    assert!(r.is_ok());
    let cnt = 1;
    let r = client
        .get_raw_transactions_v2(address, &state.last_transaction_id, cnt, false)
        .await;
    println!("{:?}", r);
    assert!(r.is_ok());
    assert_eq!(r.unwrap().transactions.len(), cnt);
    Ok(())
}

#[tokio::test]
async fn client_smc_run_get_method_works() -> anyhow::Result<()> {
    common::init_logging();
    {
        let client = common::new_test_client().await?;
        let (conn, id1) = client
            .smc_load("EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR")
            .await?; // pool 0.3.0
        let method_id = SmcMethodId::Name {
            name: "get_jetton_data".to_string(),
        };
        let r = conn.smc_run_get_method(id1, &method_id, &Vec::new()).await;
        println!("{:?}", r);
        // Check that it works after cloning the connection
        let id2 = {
            let conn2 = conn.clone();
            conn2
                .smc_load("EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR")
                .await? // pool 0.3.0
                .1
        };
        let stack = &Vec::new();
        let method_id = SmcMethodId::Name {
            name: "get_jetton_data".to_string(),
        };
        let future = conn.smc_run_get_method(id2, &method_id, stack);
        let r = timeout(Duration::from_secs(2), future).await?;
        println!("{:?}", r);
    }
    thread::sleep(Duration::from_secs(2));
    Ok(())
}

#[tokio::test]
async fn client_get_block_header_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let seqno = client.get_masterchain_info().await?.last.seqno;
    let block_id = BlockId {
        workchain: -1,
        shard: i64::MIN,
        seqno,
    };
    let block_id_ext = client.lookup_block(1, &block_id, 0, 0).await?;
    let r = client.get_block_header(&block_id_ext).await?;
    println!("{:?}", r);
    Ok(())
}

#[tokio::test]
async fn client_blocks_get_transactions() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let info: BlocksMasterchainInfo = client.get_masterchain_info().await?;
    println!("MasterchainInfo: {:?}", &info);
    let block_id = BlockId {
        workchain: info.last.workchain,
        shard: info.last.shard,
        seqno: info.last.seqno,
    };
    let block_id_ext = client.lookup_block(1, &block_id, 0, 0).await?;
    println!("BlockIdExt: {:?}", &block_id_ext);
    let block_shards: BlocksShards = client.get_block_shards(&info.last).await?;
    let mut shards = block_shards.shards.clone();
    println!("Shards: {:?}", &block_shards);
    shards.insert(0, info.last.clone());
    for shard in &shards {
        println!("Processing shard: {:?}", shard);
        let workchain = shard.workchain;
        let txs: BlocksTransactions = client
            .get_block_transactions(&shard, 7, 1024, &NULL_BLOCKS_ACCOUNT_TRANSACTION_ID)
            .await?;
        println!(
            "Number of transactions: {}, incomplete: {}",
            txs.transactions.len(),
            txs.incomplete
        );
        for tx_id in txs.transactions {
            let mut t: [u8; 32] = [0; 32];
            t.clone_from_slice(tx_id.account.as_slice());
            let addr = TonAddress::new(workchain, &t);
            let id = InternalTransactionId {
                hash: tx_id.hash.clone(),
                lt: tx_id.lt,
            };
            let tx = client
                .get_raw_transactions_v2(addr.to_hex().as_str(), &id, 1, false)
                .await?;
            println!("Tx: {:?}", tx.transactions[0])
        }
    }
    Ok(())
}
