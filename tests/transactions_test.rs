use anyhow::anyhow;
use std::sync::Arc;
use std::time::Duration;
use std::{thread, time};
use tonlib::address::TonAddress;
use tonlib::contract::{LatestContractTransactionsCache, TonContractFactory};
use tonlib::tl::RawTransaction;

mod common;

#[tokio::test]
async fn get_txs_for_frequent_works() -> anyhow::Result<()> {
    common::init_logging();
    let validator: &TonAddress = &"Ef9VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVbxn".parse()?;

    let client = common::new_test_client().await?;
    let factory = TonContractFactory::builder(&client)
        .with_cache(100, Duration::from_secs(10))
        .build()
        .await?;
    let trans = LatestContractTransactionsCache::new(&factory, validator, 100, true);
    let trs = trans.get(4).await?;
    println!(
        "Got {} transactions, first {}, last {}",
        trs.len(),
        trs.first().unwrap().transaction_id.lt,
        trs.last().unwrap().transaction_id.lt
    );
    assert_eq!(4, trs.len());
    assert_eq!(
        true,
        trs.first().unwrap().transaction_id.lt > trs.last().unwrap().transaction_id.lt
    );
    check_order(trs).expect("Invalid transactions list");

    let trs = trans.get(30).await?;
    println!(
        "Got {} transactions, first {}, last {}",
        trs.len(),
        trs.first().unwrap().transaction_id.lt,
        trs.last().unwrap().transaction_id.lt,
    );
    assert_eq!(30, trs.len());
    assert_eq!(
        true,
        trs.first().unwrap().transaction_id.lt > trs.last().unwrap().transaction_id.lt
    );
    check_order(trs).expect("Invalid transactions list");

    thread::sleep(time::Duration::from_millis(10000));

    let trs = trans.get(16).await?;
    println!(
        "Got {} transactions, first {}, last {}",
        trs.len(),
        trs.first().unwrap().transaction_id.lt,
        trs.last().unwrap().transaction_id.lt,
    );
    assert_eq!(16, trs.len());
    assert_eq!(
        true,
        trs.first().unwrap().transaction_id.lt > trs.last().unwrap().transaction_id.lt
    );
    check_order(trs).expect("Invalid transactions list");

    Ok(())
}

#[tokio::test]
async fn get_txs_for_rare_works() -> anyhow::Result<()> {
    common::init_logging();
    let addr: &TonAddress = &"EQC9kYAEZS0ePT8KCnwk6Fo69HO0t_FEqIRmIY7rW6fh3lK7".parse()?;

    let client = common::new_test_client().await?;
    let factory = TonContractFactory::builder(&client)
        .with_cache(100, Duration::from_secs(10))
        .build()
        .await?;
    let trans = LatestContractTransactionsCache::new(&factory, addr, 100, true);

    let trs = trans.get(4).await?;
    if trs.is_empty() {
        println!("Got 0 transactions");
    } else {
        println!(
            "Got {} transactions, first {:?}, last {:?}",
            trs.len(),
            trs.first().unwrap().transaction_id.lt,
            trs.last().unwrap().transaction_id.lt
        );
        assert_eq!(
            true,
            trs.first().unwrap().transaction_id.lt >= trs.last().unwrap().transaction_id.lt
        );
        check_order(trs).expect("Invalid transactions list");
    }

    let trs = trans.get(30).await?;
    if trs.is_empty() {
        println!("Got 0 transactions");
    } else {
        println!(
            "Got {} transactions, first {}, last {}",
            trs.len(),
            trs.first().unwrap().transaction_id.lt,
            trs.last().unwrap().transaction_id.lt,
        );
        assert_eq!(
            true,
            trs.first().unwrap().transaction_id.lt > trs.last().unwrap().transaction_id.lt
        );
        check_order(trs).expect("Invalid transactions list");
    }

    let mut missing_hash = addr.hash_part.clone();
    missing_hash[31] += 1;
    let missing_addr = TonAddress::new(addr.workchain, &missing_hash);
    let missing_trans = LatestContractTransactionsCache::new(&factory, &missing_addr, 100, true);
    let missing_trs = missing_trans.get(30).await?;

    assert_eq!(missing_trs.len(), 0);
    Ok(())
}

#[tokio::test]
async fn get_txs_for_empty_works() -> anyhow::Result<()> {
    common::init_logging();
    let addr: &TonAddress = &"EQAjJIyYzKc4bww1zo3_fAqHWZdYCJHwhs84wtU8smO_Hr3i".parse()?;

    let client = common::new_test_client().await?;
    let factory = TonContractFactory::builder(&client)
        .with_cache(100, Duration::from_secs(10))
        .build()
        .await?;
    let trans = LatestContractTransactionsCache::new(&factory, addr, 100, true);
    let trs = trans.get(4).await?;
    println!(
        "Got {} transactions, first {:?}, last {:?}",
        trs.len(),
        trs.first(),
        trs.last()
    );
    let trs = trans.get(30).await?;
    println!(
        "Got {} transactions, first {:?}, last {:?}",
        trs.len(),
        trs.first(),
        trs.last(),
    );

    Ok(())
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
