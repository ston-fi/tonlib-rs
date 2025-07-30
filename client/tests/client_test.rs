use std::fs::create_dir_all;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use futures::future::join_all;
use tokio::time::timeout;
use tokio::{self};
use tokio_test::assert_ok;
use tonlib_client::client::{
    TonBlockFunctions, TonClient, TonClientBuilder, TonClientInterface, TxId,
};
use tonlib_client::config::{MAINNET_CONFIG, TESTNET_CONFIG};
use tonlib_client::contract::{TonContractFactory, TonContractInterface};
use tonlib_client::tl::{
    BlockId, BlockIdExt, BlocksShards, BlocksTransactions, BlocksTransactionsExt,
    InternalTransactionId, SmcLibraryQueryExt, TonLibraryId, NULL_BLOCKS_ACCOUNT_TRANSACTION_ID,
};
use tonlib_core::cell::dict::predefined_readers::{key_reader_256bit, val_reader_cell};
use tonlib_core::cell::{ArcCell, BagOfCells, CellBuilder};
use tonlib_core::tlb_types::tlb::TLB;
use tonlib_core::{TonAddress, TonHash, TonTxId};

mod common;

#[tokio::test]
async fn test_client_get_masterchain_info() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let (_, _) = client.get_masterchain_info().await?;
    Ok(())
}

#[tokio::test]
async fn test_client_get_account_state_of_inactive() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = TonContractFactory::builder(&client).build().await?;
    let addr = TonAddress::from_str("EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR")?;
    let res = assert_ok!(factory.get_latest_account_state(&addr).await);
    log::debug!("{res:?}");
    assert!(res.frozen_hash.is_empty(), "Expected Uninitialized state");
    Ok(())
}

#[tokio::test]
async fn client_get_raw_account_state_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let r = assert_ok!(
        client
            .get_raw_account_state(assert_ok!(&TonAddress::from_base64_url(
                "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR",
            )))
            .await
    );
    log::info!("{r:?}");
    Ok(())
}

#[tokio::test]
async fn client_get_raw_account_state_by_tx_works_on_fresh_pool() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = TonContractFactory::builder(&client).build().await?;

    let address = &TonAddress::from_base64_url("EQBw_0u4LyoweyLGjyAiGg0W_wozq4S5EAQwLIsx15a4U4ar")?;
    let contract = factory.get_contract(address);
    let r = assert_ok!(contract.get_account_state().await);
    log::info!("{r:?}");
    Ok(())
}

#[tokio::test]
async fn client_get_raw_transactions_works() -> anyhow::Result<()> {
    common::init_logging();
    let address = &assert_ok!(TonAddress::from_base64_url(
        "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR"
    ));
    let max_retries = 3;
    let mut retries = 0;
    while retries < max_retries {
        retries += 1;
        let client = common::new_mainnet_client().await;
        let state = client.get_raw_account_state(address).await?;
        let r = client
            .get_raw_transactions(address, &state.last_transaction_id)
            .await;
        println!("{r:?}");
        if r.is_ok() {
            let cnt = 1;
            let r = client
                .get_raw_transactions_v2(address, &state.last_transaction_id, cnt, false)
                .await;
            println!("{r:?}");
            if r.is_ok() {
                assert_eq!(r?.transactions.len(), cnt);
                return Ok(());
            }
            assert!(client
                .get_raw_transactions_v2(address, &state.last_transaction_id, 17, false)
                .await
                .is_err());
        }
    }
    Ok(())
}

#[tokio::test]
async fn client_smc_run_get_method_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let address = &assert_ok!(TonAddress::from_base64_url(
        "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR"
    ));
    let loaded_state = assert_ok!(client.smc_load(address).await); // pool 0.3.0
    let method_id = "get_jetton_data".into();
    let conn = loaded_state.conn.clone();

    let r = loaded_state
        .conn
        .smc_run_get_method(loaded_state.id, &method_id, &Vec::new())
        .await;
    log::info!("{r:?}");
    // Check that it works after cloning the connection
    let id2 = {
        let conn2 = conn.clone();
        assert_ok!(conn2.smc_load(address).await) // pool 0.3.0
            .id
    };
    let stack = &Vec::new();
    let future = conn.smc_run_get_method(id2, &method_id, stack);
    let r = assert_ok!(timeout(Duration::from_secs(2), future).await);
    log::info!("{r:?}");
    Ok(())
}

#[tokio::test]
async fn client_smc_load_by_transaction_works() -> anyhow::Result<()> {
    common::init_logging();

    let address = &assert_ok!(TonAddress::from_base64_url(
        "EQCVx4vipWfDkf2uNhTUkpT97wkzRXHm-N1cNn_kqcLxecxT"
    ));
    let transaction_id = assert_ok!(TonTxId::from_str(
        "32016630000001:91485a21ba6eaaa91827e357378fe332228d11f3644e802f7e0f873a11ce9c6f",
    ));

    let max_retries = 3;
    let mut retries = 0;
    while retries < max_retries {
        retries += 1;
        let client = common::new_mainnet_client().await;

        let state = client.get_raw_account_state(address).await?;

        log::info!("TRANSACTION_ID{}", &state.last_transaction_id);

        let tx_id = Arc::new(TxId {
            address: address.clone(),
            internal_transaction_id: transaction_id.clone().into(),
        });
        let res = client
            .smc_load_by_transaction(&tx_id.address, &tx_id.internal_transaction_id)
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
    let client = common::new_mainnet_client().await;
    let address = &assert_ok!(TonAddress::from_base64_url(
        "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR"
    ));
    let loaded_state = assert_ok!(client.smc_load(address).await);
    let cell = assert_ok!(loaded_state.conn.smc_get_code(loaded_state.id).await);
    log::info!("\n\r\x1b[1;35m-----------------------------------------CODE-----------------------------------------\x1b[0m:\n\r {:?}", STANDARD.encode(cell.bytes));
    Ok(())
}

#[tokio::test]
async fn client_smc_get_data_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let address = &assert_ok!(TonAddress::from_base64_url(
        "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR"
    ));
    let loaded_state = assert_ok!(client.smc_load(address).await);
    let cell = assert_ok!(loaded_state.conn.smc_get_data(loaded_state.id).await);
    log::info!("\n\r\x1b[1;35m-----------------------------------------DATA-----------------------------------------\x1b[0m:\n\r {:?}", STANDARD.encode(cell.bytes));
    Ok(())
}

#[tokio::test]
async fn test_get_jetton_content_internal_uri_jusdt() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let address = &assert_ok!(TonAddress::from_base64_url(
        "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR"
    ));
    let loaded_state = assert_ok!(client.smc_load(address).await);
    let cell = assert_ok!(loaded_state.conn.smc_get_state(loaded_state.id).await);
    log::info!("\n\r\x1b[1;35m-----------------------------------------STATE----------------------------------------\x1b[0m:\n\r {cell:?}");
    Ok(())
}

#[tokio::test]
async fn client_get_block_header_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let (_, info) = assert_ok!(client.get_masterchain_info().await);
    let seqno = info.last.seqno;
    let block_id = BlockId {
        workchain: -1,
        shard: i64::MIN,
        seqno,
    };
    let block_id_ext = assert_ok!(client.lookup_block(1, &block_id, 0, 0).await);
    let r = assert_ok!(client.get_block_header(&block_id_ext).await);
    log::info!("{r:?}");
    Ok(())
}

#[tokio::test]
async fn test_client_blocks_get_transactions() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let (_, info) = assert_ok!(client.get_masterchain_info().await);
    log::info!("MasterchainInfo: {:?}", &info);
    let block_id = BlockId {
        workchain: info.last.workchain,
        shard: info.last.shard,
        seqno: info.last.seqno,
    };
    let block_id_ext = assert_ok!(client.lookup_block(1, &block_id, 0, 0).await);
    log::info!("BlockIdExt: {:?}", &block_id_ext);
    let block_shards: BlocksShards = assert_ok!(client.get_block_shards(&info.last).await);
    let mut shards = block_shards.shards.clone();
    log::info!("Shards: {:?}", &block_shards);
    shards.insert(0, info.last.clone());
    for shard in &shards {
        log::info!("Processing shard: {shard:?}");
        let workchain = shard.workchain;
        let txs: BlocksTransactions = assert_ok!(
            client
                .get_block_transactions(shard, 7, 1024, &NULL_BLOCKS_ACCOUNT_TRANSACTION_ID)
                .await
        );
        log::info!(
            "Number of transactions: {}, incomplete: {}",
            txs.transactions.len(),
            txs.incomplete
        );
        for tx_id in txs.transactions {
            let hash = TonHash::try_from(tx_id.account)?;
            let addr = TonAddress::new(workchain, hash);
            let id = InternalTransactionId {
                hash: tx_id.hash.clone(),
                lt: tx_id.lt,
            };
            let tx = assert_ok!(client.get_raw_transactions_v2(&addr, &id, 1, false).await);
            log::info!("Tx: {:?}", tx.transactions[0])
        }
    }
    Ok(())
}

#[tokio::test]
async fn test_client_blocks_get_transactions_ext() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let (_, info) = assert_ok!(client.get_masterchain_info().await);
    log::info!("MasterchainInfo: {:?}", &info);
    let block_id = BlockId {
        workchain: info.last.workchain,
        shard: info.last.shard,
        seqno: info.last.seqno,
    };
    let block_id_ext = assert_ok!(client.lookup_block(1, &block_id, 0, 0).await);
    log::info!("BlockIdExt: {:?}", &block_id_ext);
    let block_shards: BlocksShards = assert_ok!(client.get_block_shards(&info.last).await);
    let mut shards = block_shards.shards.clone();
    log::info!("Shards: {:?}", &block_shards);
    shards.insert(0, info.last.clone());
    for shard in &shards {
        log::info!("Processing shard: {shard:?}");
        let txs: BlocksTransactionsExt = assert_ok!(
            client
                .get_block_transactions_ext(shard, 7, 1024, &NULL_BLOCKS_ACCOUNT_TRANSACTION_ID)
                .await
        );
        log::info!(
            "Number of transactions: {}, incomplete: {}",
            txs.transactions.len(),
            txs.incomplete
        );
        for raw_tx in txs.transactions {
            let addr = assert_ok!(TonAddress::from_base64_url(
                raw_tx.address.account_address.as_str()
            ));
            let id = raw_tx.transaction_id;
            let tx = assert_ok!(client.get_raw_transactions_v2(&addr, &id, 1, false).await);
            log::info!("Tx: {:?}", tx.transactions[0])
        }
    }
    Ok(())
}

#[tokio::test]
async fn test_client_lite_server_get_info() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let info = assert_ok!(client.lite_server_get_info().await);
    assert!(info.now > 0);
    log::info!("{info:?}");
    Ok(())
}

#[tokio::test]
async fn test_get_config_param() -> anyhow::Result<()> {
    common::init_logging();
    let client = &common::new_mainnet_client().await;
    let info = assert_ok!(client.get_config_param(0u32, 34u32).await);
    let config_data = info.config.bytes;
    let config_cell = ArcCell::from_boc(&config_data)?;
    let mut parser = config_cell.parser();
    let n = assert_ok!(parser.load_u8(8));
    assert_eq!(n, 0x12u8);
    Ok(())
}

#[tokio::test]
pub async fn test_get_block_header() -> anyhow::Result<()> {
    common::init_logging();
    let client = &common::new_mainnet_client().await;
    let (_, info) = assert_ok!(client.get_masterchain_info().await);
    let seqno = info.last;
    let headers = assert_ok!(client.get_block_header(&seqno).await);
    assert_eq!(headers.id, seqno);
    log::info!("{headers:?}");
    Ok(())
}

#[tokio::test]
async fn test_get_shard_tx_ids() -> anyhow::Result<()> {
    common::init_logging();
    let client = &common::new_mainnet_client().await;
    let (_, info) = assert_ok!(client.get_masterchain_info().await);
    let shards = assert_ok!(client.get_block_shards(&info.last).await);
    assert!(!shards.shards.is_empty());
    let ids = assert_ok!(client.get_shard_tx_ids(&shards.shards[0]).await);
    assert!(!ids.is_empty());
    log::debug!("{ids:?}");
    Ok(())
}

#[tokio::test]
async fn test_get_shard_transactions_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = &common::new_mainnet_client().await;
    let (_, info) = assert_ok!(client.get_masterchain_info().await);
    let shards = assert_ok!(client.get_block_shards(&info.last).await);
    assert!(!shards.shards.is_empty());
    let txs = assert_ok!(client.get_shard_transactions(&shards.shards[0]).await);
    assert!(!txs.is_empty());
    log::debug!("{txs:?}");
    Ok(())
}

#[tokio::test]
async fn test_get_shard_transactions_parse_address_correctly() -> anyhow::Result<()> {
    common::init_logging();
    let client = &common::new_mainnet_client().await;
    client.sync().await?;
    // manually selected block with particular addresses format in transactions
    let block_shard = BlockIdExt {
        workchain: -1,
        shard: -9223372036854775808,
        seqno: 39812357,
        root_hash: STANDARD.decode("WFgmnfd3wuQR9HydL54EjcuDvLYM/SIwDbDxbNzDyjU=")?,
        file_hash: STANDARD.decode("scgMz5C3n0uBeb2pdf2e8/BWlfzTB8FcRsNvvHgXKYM=")?,
    };
    let txs = assert_ok!(client.get_shard_transactions(&block_shard).await);
    assert!(!txs.is_empty());

    log::info!("{txs:?}");
    let not_a_block_shard = BlockIdExt {
        workchain: -1,
        shard: -9223372036854775808,
        seqno: 39812359,
        root_hash: STANDARD.decode("WFgmnfd3wuQR9HydL54EjcuDvLYM/SIwDbDxbNzDyjU=")?,
        file_hash: STANDARD.decode("scgMz5C3n0uBeb2pdf2e8/BWlfzTB8FcRsNvvHgXKYM=")?,
    };

    assert!(client
        .get_shard_transactions(&not_a_block_shard)
        .await
        .is_err());
    Ok(())
}

#[tokio::test]
async fn test_get_shards_transactions() -> anyhow::Result<()> {
    common::init_logging();
    let client = &common::new_mainnet_client().await;
    let (_, info) = client.get_masterchain_info().await?;
    let shards = assert_ok!(client.get_block_shards(&info.last).await);
    assert!(!shards.shards.is_empty());
    let shards_txs = assert_ok!(client.get_shards_transactions(&shards.shards).await);
    assert!(!shards_txs.is_empty());
    for s in shards_txs {
        assert!(!s.1.is_empty());
        log::info!("{:?} : {:?}", s.0, s.1);
    }
    Ok(())
}

#[tokio::test]
async fn test_missing_block_error() -> anyhow::Result<()> {
    common::init_logging();
    let client = &common::new_mainnet_client().await;
    let (_, info) = client.get_masterchain_info().await?;
    let block_id = BlockId {
        workchain: info.last.workchain,
        shard: info.last.shard,
        seqno: info.last.seqno + 2,
    };
    for _i in 0..100 {
        let res = client.lookup_block(1, &block_id, 0, 0).await;
        log::info!("{res:?}");
        tokio::time::sleep(Duration::from_millis(10)).await;
        if res.is_ok() {
            break;
        };
    }
    Ok(())
}

#[tokio::test]
async fn test_first_block_error() -> anyhow::Result<()> {
    common::init_logging();
    let client = &common::new_mainnet_client_archive().await;
    let (_, info) = client.get_masterchain_info().await?;
    let block_id = BlockId {
        workchain: info.last.workchain,
        shard: info.last.shard,
        seqno: 1,
    };
    let res = client.lookup_block(1, &block_id, 0, 0).await;
    log::info!("{res:?}");
    assert_ok!(res);
    Ok(())
}

#[tokio::test]
async fn test_keep_connection_alive() -> anyhow::Result<()> {
    common::init_logging();
    let client = &common::new_mainnet_client_archive().await;
    let (_, info) = client.get_masterchain_info().await?;
    let next_block_id = BlockId {
        workchain: info.last.workchain,
        shard: info.last.shard,
        seqno: info.last.seqno + 10,
    };
    let first_block_id = BlockId {
        workchain: -1,
        shard: i64::MIN,
        seqno: 1,
    };
    let conn = assert_ok!(client.get_connection().await);
    let r1 = conn.lookup_block(1, &first_block_id, 0, 0).await;
    log::info!("R1: {r1:?}");
    let r2 = conn.lookup_block(1, &next_block_id, 0, 0).await;
    log::info!("R1: {r2:?}");
    let r3 = conn.lookup_block(1, &first_block_id, 0, 0).await;
    log::info!("R1: {r3:?}");
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(())
}

#[tokio::test]
async fn client_mainnet_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = assert_ok!(
        TonClient::builder()
            .with_pool_size(2)
            .with_config(&MAINNET_CONFIG)
            .build()
            .await
    );
    let (_, info) = client.get_masterchain_info().await?;
    let shards = client.get_block_shards(&info.last).await?;
    let blocks_header = client.get_block_header(&info.last).await?;
    assert!(!shards.shards.is_empty());
    let shards_txs = client.get_shards_transactions(&shards.shards).await?;
    for s in shards_txs {
        log::info!(" BlockId: {:?}\n Transactions: {:?}", s.0, s.1.len());
    }
    log::info!(
        "MAINNET: Blocks header for  {} seqno : {:?}",
        info.last.seqno,
        blocks_header
    );
    Ok(())
}

#[tokio::test]
async fn client_testnet_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = assert_ok!(
        TonClient::builder()
            .with_pool_size(2)
            .with_config(&TESTNET_CONFIG)
            .build()
            .await
    );
    let (_, info) = client.get_masterchain_info().await?;
    let shards = client.get_block_shards(&info.last).await?;
    assert!(!shards.shards.is_empty());
    Ok(())
}

#[tokio::test]
async fn client_smc_get_libraries() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let library_hash_str = "TwFxJywhW4v4/urEaoV2iKS2X0/mH4IoYx9ifQ7anQA=";
    let library_hash = TonLibraryId {
        id: STANDARD.decode(library_hash_str)?,
    };

    let library_list = &[library_hash];
    let smc_library_result = client.smc_get_libraries(library_list).await?;

    log::info!(
        "smc_library_result {:?}",
        STANDARD.encode(smc_library_result.result[0].hash.clone())
    );
    assert_eq!(
        STANDARD.encode(smc_library_result.result[0].hash.clone()),
        library_hash_str
    );

    // we just test that library code is a valid boc:
    let boc = BagOfCells::parse(smc_library_result.result[0].data.as_slice())?;
    log::info!("smc_library_result {boc:?}");
    Ok(())
}

#[tokio::test]
async fn client_smc_get_libraries_ext() -> anyhow::Result<()> {
    common::init_logging();

    let client = common::new_mainnet_client().await;

    let address = assert_ok!(TonAddress::from_base64_url(
        "EQDqVNU7Jaf85MhIba1lup0F7Mr3rGigDV8RxMS62RtFr1w8"
    )); //jetton master
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract = factory.get_contract(&address);
    let code = &assert_ok!(contract.get_account_state().await).code;
    let library_query = SmcLibraryQueryExt::ScanBoc {
        boc: code.clone(),
        max_libs: 10,
    };

    let library_hash = "TwFxJywhW4v4/urEaoV2iKS2X0/mH4IoYx9ifQ7anQA=";

    let smc_libraries_ext_result = assert_ok!(client.smc_get_libraries_ext(&[library_query]).await);

    log::info!("smc_libraries_ext_result {smc_libraries_ext_result:?}");

    assert_eq!(1, smc_libraries_ext_result.libs_ok.len());
    assert_eq!(0, smc_libraries_ext_result.libs_not_found.len());
    assert_eq!(
        smc_libraries_ext_result.libs_ok[0].id,
        assert_ok!(STANDARD.decode(library_hash))
    );

    let boc = BagOfCells::parse(&smc_libraries_ext_result.dict_boc)?;
    let cell_ref = boc.single_root()?;
    let mut cell_builder = CellBuilder::new();
    cell_builder.store_bit(true)?;
    cell_builder.store_reference(&cell_ref)?;
    let cell = cell_builder.build()?;

    let dict = assert_ok!(cell
        .parser()
        .load_dict(256, key_reader_256bit, val_reader_cell));

    log::info!("DICT: {dict:?}");

    assert_eq!(dict.len(), 1);
    assert!(dict.contains_key(&TonHash::try_from(STANDARD.decode(library_hash)?)?));
    Ok(())
}

// This test fails on tonlib 2023.6, 2024.1 and 2024.3 either with:
//   error: test failed, to rerun pass `-p tonlib --test client_test`
//     Caused by:
//     process didn't exit successfully: `../target/debug/deps/client_test-a6ec52f42b3d3962 dropping_invoke_test --exact --nocapture --ignored`
//    (signal: 6, SIGABRT: process abort signal)
//  or:
//   error: test failed, to rerun pass `-p tonlib --test client_test`
//     Caused by:
//     process didn't exit successfully: `../target/debug/deps/client_test-a6ec52f42b3d3962 dropping_invoke_test --exact --nocapture --ignored`
//     (signal: 11, SIGSEGV: invalid memory reference)
#[ignore]
#[tokio::test]
async fn dropping_invoke_test() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let address = assert_ok!(TonAddress::from_base64_url(
        "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR"
    ));
    client.get_raw_account_state(&address).await?;

    let f = [
        abort_batch_invoke_get_raw_account_state(&client, Duration::from_millis(100)),
        abort_batch_invoke_get_raw_account_state(&client, Duration::from_millis(200)),
        abort_batch_invoke_get_raw_account_state(&client, Duration::from_millis(500)),
    ];

    join_all(f).await;
    Ok(())
}

async fn abort_batch_invoke_get_raw_account_state(
    client: &TonClient,
    dt: Duration,
) -> anyhow::Result<()> {
    let address = assert_ok!(TonAddress::from_base64_url(
        "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR"
    ));
    let addresses = vec![address; 100];

    let futures = addresses
        .iter()
        .map(|a| timeout(dt, client.get_raw_account_state(a)))
        .collect::<Vec<_>>();

    let result = join_all(futures).await;

    let res = result.iter().map(|r| r.is_ok()).collect::<Vec<_>>();
    log::info!("{res:?}");
    Ok(())
}

#[tokio::test]
async fn archive_node_client_test() -> anyhow::Result<()> {
    let tonlib_work_dir = "./var/tonlib";
    create_dir_all(Path::new(tonlib_work_dir))?;
    TonClient::set_log_verbosity_level(2);

    let mut client_builder = TonClientBuilder::new();
    client_builder
        .with_config(&MAINNET_CONFIG)
        .with_keystore_dir(String::from(tonlib_work_dir))
        .with_connection_check(tonlib_client::client::ConnectionCheck::Archive);
    let client = client_builder.build().await?;
    let (_, master_info) = client.get_masterchain_info().await?;
    log::info!("master_info: {master_info:?}");
    Ok(())
}
