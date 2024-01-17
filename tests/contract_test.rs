use std::thread;
use std::time::Duration;

use anyhow::anyhow;
use async_trait::async_trait;
use num_bigint::BigUint;

use tonlib::address::TonAddress;
use tonlib::contract::{TonContractFactory, TonContractInterface, TonContractState};

mod common;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PoolData {
    pub reserve0: BigUint,
    pub reserve1: BigUint,
    pub token0_address: TonAddress,
    pub token1_address: TonAddress,
    pub lp_fee: i32,
    pub protocol_fee: i32,
    pub ref_fee: i32,
    pub protocol_fee_address: TonAddress,
    pub collected_token0_protocol_fee: BigUint,
    pub collected_token1_protocol_fee: BigUint,
}

#[async_trait]
pub trait PoolContract: TonContractInterface {
    async fn get_pool_data(&self) -> anyhow::Result<PoolData> {
        let res = self.run_get_method("get_pool_data", &Vec::new()).await?;
        if res.stack.elements.len() == 10 {
            let pool_data = PoolData {
                reserve0: res.stack.get_biguint(0)?,
                reserve1: res.stack.get_biguint(1)?,
                token0_address: res
                    .stack
                    .get_boc(2)?
                    .single_root()?
                    .parse_fully(|r| r.load_address())?,
                token1_address: res
                    .stack
                    .get_boc(3)?
                    .single_root()?
                    .parse_fully(|r| r.load_address())?,
                lp_fee: res.stack.get_i32(4)?,
                protocol_fee: res.stack.get_i32(5)?,
                ref_fee: res.stack.get_i32(6)?,
                protocol_fee_address: res
                    .stack
                    .get_boc(7)?
                    .single_root()?
                    .parse_fully(|r| r.load_address())?,
                collected_token0_protocol_fee: res.stack.get_biguint(8)?,
                collected_token1_protocol_fee: res.stack.get_biguint(9)?,
            };
            Ok(pool_data)
        } else {
            Err(anyhow!(
                "Invalid result size: {}, expected 10",
                res.stack.elements.len()
            ))
        }
    }

    async fn invalid_method(&self) -> anyhow::Result<()> {
        let _ = self.run_get_method("invalid_method", &Vec::new()).await?;
        Ok(())
    }
}

impl<T> PoolContract for T where T: TonContractInterface {}

#[tokio::test]
async fn contract_get_pool_data_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await?;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        factory.get_contract(&"EQD9b5pxv6nptJmD1-c771oRV98h_mky-URkDn5BJpY2sTJ-".parse()?);
    let pool_data = contract.get_pool_data().await?;
    println!("pool data: {:?}", pool_data);
    let invalid_result = contract.invalid_method().await;
    assert!(invalid_result.is_err());
    Ok(())
}

#[tokio::test]
async fn state_get_pool_data_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await?;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        factory.get_contract(&"EQD9b5pxv6nptJmD1-c771oRV98h_mky-URkDn5BJpY2sTJ-".parse()?);
    let state = contract.get_state().await?;
    let pool_data = state.get_pool_data().await?;
    println!("pool data: {:?}", pool_data);
    let invalid_result = contract.invalid_method().await;
    assert!(invalid_result.is_err());
    Ok(())
}

#[tokio::test]
async fn state_clone_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await?;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        factory.get_contract(&"EQD9b5pxv6nptJmD1-c771oRV98h_mky-URkDn5BJpY2sTJ-".parse()?);
    let state1 = contract.get_state().await?;
    let pool_data = state1.get_pool_data().await?;
    println!("pool data: {:?}", pool_data);
    {
        let state2 = state1.clone();
        let pool_data = state2.get_pool_data().await?;
        println!("pool data: {:?}", pool_data);
    }
    tokio::time::sleep(Duration::from_millis(1000)).await;
    let pool_data = state1.get_pool_data().await?;
    println!("pool data: {:?}", pool_data);
    Ok(())
}

#[tokio::test]
async fn test_acoount_state_by_transaction() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await?;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        factory.get_contract(&"EQB3ncyBUTjZUA5EnFKR5_EnOMI9V1tTEAAPaiU71gc4TiUt".parse()?);
    let state1 = contract.get_account_state().await?;
    log::info!(
        "Loading state {} for {}",
        state1.last_transaction_id,
        contract.address()
    );
    let state2 = contract
        .get_account_state_by_transaction(&state1.last_transaction_id)
        .await?;
    // Not testing equality of block_id & sync_utime since they are not really a part of contract state
    assert_eq!(state1.balance, state2.balance);
    assert_eq!(state1.code, state2.code);
    assert_eq!(state1.data, state2.data);
    assert_eq!(state1.last_transaction_id, state2.last_transaction_id);
    assert_eq!(state1.frozen_hash, state2.frozen_hash);
    Ok(())
}

#[tokio::test]
async fn test_contract_state_by_transaction() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await?;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        factory.get_contract(&"EQB3ncyBUTjZUA5EnFKR5_EnOMI9V1tTEAAPaiU71gc4TiUt".parse()?);
    let method_name = "get_router_data";
    let account_state = contract.get_account_state().await?;
    log::info!(
        "Using state {} for {}",
        account_state.last_transaction_id,
        contract.address()
    );
    let contract_state1 = contract.get_state().await?;
    let contract_state2 = contract
        .get_state_by_transaction(&account_state.last_transaction_id)
        .await?;
    let result1 = contract_state1.run_get_method(method_name, &vec![]).await?;
    let result2 = contract_state2.run_get_method(method_name, &vec![]).await?;
    assert_eq!(result1, result2);
    Ok(())
}

#[tokio::test]
async fn test_state_dropping() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await?;
    let factory = TonContractFactory::builder(&client).build().await?;
    let state = factory
        .get_contract_state(&"EQB3ncyBUTjZUA5EnFKR5_EnOMI9V1tTEAAPaiU71gc4TiUt".parse()?)
        .await?;
    let thread_builder = thread::Builder::new().name("test_drop".to_string());
    let handle = thread_builder.spawn(move || test_drop(state))?;
    log::info!("Dropping state");
    let r = handle.join();
    log::info!("Join result: {:?}", r);
    assert!(r.is_ok());
    Ok(())
}

fn test_drop(state: TonContractState) {
    drop(state);
}
