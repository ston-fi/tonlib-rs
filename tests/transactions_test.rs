use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use std::{thread, time};

use anyhow::anyhow;
use futures::future::join_all;
use tokio_test::assert_ok;
use tonlib::address::TonAddress;
use tonlib::contract::{LatestContractTransactionsCache, TonContractFactory};
use tonlib::tl::RawTransaction;

mod common;

#[tokio::test]
async fn get_txs_for_frequent_works() {
    common::init_logging();
    let validator: &TonAddress =
        &assert_ok!("Ef9VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVbxn".parse());

    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let trans = LatestContractTransactionsCache::new(&factory, validator, 100, true);
    let trs = assert_ok!(trans.get(4).await);
    log::info!(
        "Got {} transactions, first {}, last {}",
        trs.len(),
        trs.first().unwrap().transaction_id.lt,
        trs.last().unwrap().transaction_id.lt
    );
    assert_eq!(4, trs.len());
    assert!(trs.first().unwrap().transaction_id.lt > trs.last().unwrap().transaction_id.lt);
    check_order(trs).expect("Invalid transactions list");

    let trs = assert_ok!(trans.get(30).await);
    log::info!(
        "Got {} transactions, first {}, last {}",
        trs.len(),
        trs.first().unwrap().transaction_id.lt,
        trs.last().unwrap().transaction_id.lt,
    );
    assert_eq!(30, trs.len());
    assert!(trs.first().unwrap().transaction_id.lt > trs.last().unwrap().transaction_id.lt);
    check_order(trs).expect("Invalid transactions list");

    thread::sleep(time::Duration::from_millis(10000));

    let trs = assert_ok!(trans.get(16).await);
    log::info!(
        "Got {} transactions, first {}, last {}",
        trs.len(),
        trs.first().unwrap().transaction_id.lt,
        trs.last().unwrap().transaction_id.lt,
    );
    assert_eq!(16, trs.len());
    assert!(trs.first().unwrap().transaction_id.lt > trs.last().unwrap().transaction_id.lt);
    check_order(trs).expect("Invalid transactions list");
}

#[tokio::test]
async fn get_txs_for_rare_works() {
    common::init_logging();
    let addr: &TonAddress = &assert_ok!("EQC9kYAEZS0ePT8KCnwk6Fo69HO0t_FEqIRmIY7rW6fh3lK7".parse());

    let client = common::new_archive_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let trans = LatestContractTransactionsCache::new(&factory, addr, 100, true);

    let trs = assert_ok!(trans.get(4).await);
    if trs.is_empty() {
        log::info!("Got 0 transactions");
    } else {
        log::info!(
            "Got {} transactions, first {:?}, last {:?}",
            trs.len(),
            trs.first().unwrap().transaction_id.lt,
            trs.last().unwrap().transaction_id.lt
        );
        assert!(trs.first().unwrap().transaction_id.lt >= trs.last().unwrap().transaction_id.lt);
        check_order(trs).expect("Invalid transactions list");
    }

    let trs = assert_ok!(trans.get(30).await);
    if trs.is_empty() {
        log::info!("Got 0 transactions");
    } else {
        log::info!(
            "Got {} transactions, first {}, last {}",
            trs.len(),
            trs.first().unwrap().transaction_id.lt,
            trs.last().unwrap().transaction_id.lt,
        );
        assert!(trs.first().unwrap().transaction_id.lt > trs.last().unwrap().transaction_id.lt);
        check_order(trs).expect("Invalid transactions list");
    }

    let mut missing_hash = addr.hash_part;
    missing_hash[31] += 1;
    let missing_addr = TonAddress::new(addr.workchain, &missing_hash);
    let missing_trans = LatestContractTransactionsCache::new(&factory, &missing_addr, 100, true);
    let missing_trs = assert_ok!(missing_trans.get(30).await);

    assert_eq!(missing_trs.len(), 0);
}

#[tokio::test]
async fn get_txs_for_empty_works() {
    common::init_logging();
    let addr: &TonAddress = &assert_ok!("EQAjJIyYzKc4bww1zo3_fAqHWZdYCJHwhs84wtU8smO_Hr3i".parse());

    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let trans = LatestContractTransactionsCache::new(&factory, addr, 100, true);
    let trs = assert_ok!(trans.get(4).await);
    log::info!(
        "Got {} transactions, first {:?}, last {:?}",
        trs.len(),
        trs.first(),
        trs.last()
    );
    let trs = assert_ok!(trans.get(30).await);
    log::info!(
        "Got {} transactions, first {:?}, last {:?}",
        trs.len(),
        trs.first(),
        trs.last(),
    );
}

fn check_order(trs: Vec<Arc<RawTransaction>>) -> anyhow::Result<()> {
    let mut lt = 0;
    for t in trs.iter() {
        if lt > 0 && t.transaction_id.lt > lt {
            return Err(anyhow!("Order is invalid for trx: {}", t.transaction_id.lt));
        }
        if lt > 0 && t.transaction_id.lt == lt {
            return Err(anyhow!("Duplicated trx: {}", t.transaction_id.lt));
        }
        lt = t.transaction_id.lt;
    }
    Ok(())
}

#[ignore]
#[tokio::test]
async fn latest_tx_data_cache_test() -> anyhow::Result<()> {
    common::init_logging();

    let client = common::new_mainnet_client().await;
    let contract_factory = TonContractFactory::builder(&client).build().await?;

    let contract_address =
        TonAddress::from_str("EQB3ncyBUTjZUA5EnFKR5_EnOMI9V1tTEAAPaiU71gc4TiUt")?;

    let capacity = 500;
    let soft_limit = true;
    let cache = LatestContractTransactionsCache::new(
        &contract_factory,
        &contract_address,
        capacity,
        soft_limit,
    );
    log::info!("Created cache");
    for _ in 0..10 {
        cache.get(capacity).await?;
    }

    let t = Instant::now();
    let mut fut = vec![];
    for j in 0..100 {
        fut.push(cache.get(j));
    }
    let r = join_all(fut).await;

    let dt = Instant::now() - t;

    for (i, t) in r.into_iter().enumerate() {
        assert_eq!(i, t?.len());
    }

    log::info!(
        "100 parallel calls to latest tx data size 500 cache takes {:?}",
        dt
    );
    drop(cache);

    Ok(())
}
