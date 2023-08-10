use std::time::Duration;
use std::{str::FromStr, thread};

use tokio;
use tokio::time::timeout;

use tonlib::cell::BagOfCells;
use tonlib::client::TonFunctions;
use tonlib::tl::types::{
    AccountState, BlockId, BlocksMasterchainInfo, BlocksShards, BlocksTransactions,
    InternalTransactionId, SmcMethodId, NULL_BLOCKS_ACCOUNT_TRANSACTION_ID,
};
use tonlib::{address::TonAddress, tl::types::LiteServerInfo};

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
    let max_retries = 3;
    let mut retries = 0;
    while retries < max_retries {
        retries += 1;
        let client = common::new_test_client().await?;
        let state = client.get_raw_account_state(address).await.unwrap();
        let r = client
            .get_raw_transactions(address, &state.last_transaction_id)
            .await;
        println!("{:?}", r);
        if r.is_ok() {
            let cnt = 1;
            let r = client
                .get_raw_transactions_v2(address, &state.last_transaction_id, cnt, false)
                .await;
            println!("{:?}", r);
            if r.is_ok() {
                assert_eq!(r.unwrap().transactions.len(), cnt);
                return Ok(());
            }
        }
    }
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
async fn client_smc_load_by_transaction_works() -> anyhow::Result<()> {
    common::init_logging();

    let address = "EQCVx4vipWfDkf2uNhTUkpT97wkzRXHm-N1cNn_kqcLxecxT";
    let internal_transaction_id = InternalTransactionId::from_str(
        "32016630000001:91485a21ba6eaaa91827e357378fe332228d11f3644e802f7e0f873a11ce9c6f",
    )?;

    let max_retries = 3;
    let mut retries = 0;
    while retries < max_retries {
        retries += 1;
        let client = common::new_test_client().await?;

        let state = client.get_raw_account_state(address).await.unwrap();

        println!("TRANSACTION_ID{}", &state.last_transaction_id);
        let res = client
            .smc_load_by_transaction(address, &internal_transaction_id)
            .await;

        if res.is_ok() {
            return Ok(());
        }
    }

    Ok(())
}

#[tokio::test]
async fn client_smc_get_code_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let address = "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR";
    let (conn, id1) = client.smc_load(address).await?;
    let cell = conn.smc_get_code(id1).await?;
    println!("\n\r\x1b[1;35m-----------------------------------------CODE-----------------------------------------\x1b[0m:\n\r {:?}",cell);
    Ok(())
}

#[tokio::test]
async fn client_smc_get_data_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let address = "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR";
    let (conn, id1) = client.smc_load(address).await?;
    let cell = conn.smc_get_data(id1).await?;
    println!("\n\r\x1b[1;35m-----------------------------------------DATA-----------------------------------------\x1b[0m:\n\r {:?}",cell);
    Ok(())
}

#[tokio::test]
async fn client_smc_get_state_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let address = "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR";
    let (conn, id1) = client.smc_load(address).await?;
    let cell = conn.smc_get_state(id1).await?;
    println!("\n\r\x1b[1;35m-----------------------------------------STATE----------------------------------------\x1b[0m:\n\r {:?}",cell);
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

#[tokio::test]
async fn client_lite_server_get_info() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let info: LiteServerInfo = client.lite_server_get_info().await?;

    println!("{:?}", info);
    Ok(())
}

#[tokio::test]
async fn test_config_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = &common::new_test_client().await?;
    let info = client.get_config_param(0u32, 34u32).await?;
    let config_data = info.config.bytes;
    let bag = BagOfCells::parse(config_data.as_slice())?;
    let config_cell = bag.single_root()?;
    let mut parser = config_cell.parser();
    let n = parser.load_u8(8)?;
    assert!(n == 0x12u8);
    Ok(())
}
