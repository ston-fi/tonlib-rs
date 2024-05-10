use std::thread;
use std::time::Duration;

use anyhow::anyhow;
use async_trait::async_trait;
use num_bigint::BigUint;
use tokio_test::assert_ok;
use tonlib::address::TonAddress;
use tonlib::contract::{
    TonContractError, TonContractFactory, TonContractInterface, TonContractState,
};
use tonlib::mnemonic::Mnemonic;
use tonlib::types::TvmSuccess;
use tonlib::wallet::{TonWallet, WalletVersion};

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
        let res = assert_ok!(self.run_get_method("get_pool_data", &Vec::new()).await);
        if res.stack.len() == 10 {
            let pool_data = PoolData {
                reserve0: assert_ok!(res.stack[0].get_biguint()),
                reserve1: assert_ok!(res.stack[1].get_biguint()),
                token0_address: assert_ok!(res.stack[2].get_address()),
                token1_address: assert_ok!(res.stack[3].get_address()),
                lp_fee: assert_ok!(res.stack[4].get_i64()) as i32,
                protocol_fee: assert_ok!(res.stack[5].get_i64()) as i32,
                ref_fee: assert_ok!(res.stack[6].get_i64()) as i32,
                protocol_fee_address: assert_ok!(res.stack[7].get_address()),
                collected_token0_protocol_fee: assert_ok!(res.stack[8].get_biguint()),
                collected_token1_protocol_fee: assert_ok!(res.stack[9].get_biguint()),
            };
            Ok(pool_data)
        } else {
            Err(anyhow!(
                "Invalid result size: {}, expected 10",
                res.stack.len()
            ))
        }
    }

    async fn invalid_method(&self) -> Result<TvmSuccess, TonContractError> {
        self.run_get_method("invalid_method", &Vec::new()).await
    }
}

impl<T> PoolContract for T where T: TonContractInterface {}

#[tokio::test]
async fn contract_get_pool_data_works() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQD9b5pxv6nptJmD1-c771oRV98h_mky-URkDn5BJpY2sTJ-".parse()
    ));
    let pool_data = assert_ok!(contract.get_pool_data().await);
    log::info!("pool data: {:?}", pool_data);
    let invalid_result = contract.invalid_method().await;
    log::info!("invalid_result: {:?}", invalid_result);

    match invalid_result {
        Ok(_) => panic!(),
        Err(err) => match err {
            TonContractError::TvmRunError { exit_code, .. } => assert_eq!(exit_code, 11),
            _ => assert_eq!(0, 1),
        },
    }
}

#[tokio::test]
async fn state_get_pool_data_works() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQD9b5pxv6nptJmD1-c771oRV98h_mky-URkDn5BJpY2sTJ-".parse()
    ));
    let state = assert_ok!(contract.get_state().await);
    let pool_data = assert_ok!(state.get_pool_data().await);
    log::info!("pool data: {:?}", pool_data);
    let invalid_result = contract.invalid_method().await;
    log::info!("Result of calling invalid method {:?}", invalid_result);
    assert!(invalid_result.is_err());
}

#[tokio::test]
async fn state_clone_works() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQD9b5pxv6nptJmD1-c771oRV98h_mky-URkDn5BJpY2sTJ-".parse()
    ));
    let state1 = assert_ok!(contract.get_state().await);
    let pool_data = assert_ok!(state1.get_pool_data().await);
    log::info!("pool data: {:?}", pool_data);
    {
        let state2 = state1.clone();
        let pool_data = assert_ok!(state2.get_pool_data().await);
        log::info!("pool data: {:?}", pool_data);
    }
    tokio::time::sleep(Duration::from_millis(1000)).await;
    let pool_data = assert_ok!(state1.get_pool_data().await);
    log::info!("pool data: {:?}", pool_data);
}

#[tokio::test]
async fn test_acoount_state_by_transaction() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQB3ncyBUTjZUA5EnFKR5_EnOMI9V1tTEAAPaiU71gc4TiUt".parse()
    ));
    let state1 = assert_ok!(contract.get_account_state().await);
    log::info!(
        "Loading state {} for {}",
        state1.last_transaction_id,
        contract.address()
    );
    let state2 = assert_ok!(
        contract
            .get_account_state_by_transaction(&state1.last_transaction_id)
            .await
    );
    // Not testing equality of block_id & sync_utime since they are not really a part of contract state
    assert_eq!(state1.balance, state2.balance);
    assert_eq!(state1.code, state2.code);
    assert_eq!(state1.data, state2.data);
    assert_eq!(state1.last_transaction_id, state2.last_transaction_id);
    assert_eq!(state1.frozen_hash, state2.frozen_hash);
}

#[tokio::test]
async fn test_contract_state_by_transaction() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQB3ncyBUTjZUA5EnFKR5_EnOMI9V1tTEAAPaiU71gc4TiUt".parse()
    ));
    let method_name = "get_router_data";
    let account_state = assert_ok!(contract.get_account_state().await);
    log::info!(
        "Using state {} for {}",
        account_state.last_transaction_id,
        contract.address()
    );
    let contract_state1 = assert_ok!(contract.get_state().await);
    let contract_state2 = assert_ok!(
        contract
            .get_state_by_transaction(&account_state.last_transaction_id)
            .await
    );
    let result1 = assert_ok!(contract_state1.run_get_method(method_name, vec![]).await);
    let result2 = assert_ok!(contract_state2.run_get_method(method_name, vec![]).await);
    assert_eq!(result1.stack, result2.stack);
}

#[tokio::test]
async fn test_state_dropping() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let state = assert_ok!(
        factory
            .get_latest_contract_state(&assert_ok!(
                "EQB3ncyBUTjZUA5EnFKR5_EnOMI9V1tTEAAPaiU71gc4TiUt".parse()
            ))
            .await
    );
    let thread_builder = thread::Builder::new().name("test_drop".to_string());
    let handle = assert_ok!(thread_builder.spawn(move || test_drop(state)));
    log::info!("Dropping state");
    let r = handle.join();
    log::info!("Join result: {:?}", r);
    assert_ok!(r);
}

fn test_drop(state: TonContractState) {
    drop(state);
}

#[tokio::test]
async fn test_derive_undeployed() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);

    let mnemonic_str = "mechanic sudden cannon bind monkey brown moment able street pride struggle team outdoor canyon coin tourist service second crazy tank sell regret sample attitude";
    let mnemonic = assert_ok!(Mnemonic::from_str(mnemonic_str, &None));
    let key_pair = assert_ok!(mnemonic.to_key_pair());
    let wallet_v4r2 = assert_ok!(TonWallet::derive_default(WalletVersion::V4R2, &key_pair));

    let address = wallet_v4r2.address;
    log::info!("addr: {}", address);
    let contract = factory.get_contract(&address);

    let r = contract.run_get_method("seqno", vec![]).await;
    log::info!("result: {:?}", r);
    assert!(r.is_err());
}
