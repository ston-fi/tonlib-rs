mod common;

use std::str::FromStr;
use std::time::{Duration, Instant};

use tokio_test::assert_ok;
use tonlib_client::contract::{
    JettonData, JettonMasterContract, JettonWalletContract, TonContractError, TonContractFactory,
    TonContractInterface,
};
use tonlib_client::emulator::c7_register::TvmEmulatorC7;
use tonlib_client::emulator::tvm_emulator::TvmEmulator;
use tonlib_client::meta::MetaDataContent;
use tonlib_client::tl::RawFullAccountState;
use tonlib_client::types::{TonMethodId, TvmStackEntry, TvmSuccess};
use tonlib_core::cell::{BagOfCells, CellBuilder, CellSlice};
use tonlib_core::{TonAddress, TonTxId};

use crate::common::new_contract_factory;

#[tokio::test]
async fn test_emulator_get_jetton_data() -> anyhow::Result<()> {
    let factory = new_contract_factory(false, false).await?;
    let address = TonAddress::from_str("EQDCJL0iQHofcBBvFBHdVG233Ri2V4kCNFgfRT-gqAd3Oc86")?; //jetton master
    let contract = factory.get_contract(&address);
    let state = contract.get_account_state().await?;

    let emulated_data = emulate_get_jetton_data(&state, &factory, &address).await?;
    let blockchain_data = contract.get_jetton_data().await?;

    assert_eq!(blockchain_data.total_supply, emulated_data.total_supply);
    assert_eq!(blockchain_data.mintable, emulated_data.mintable);
    assert_eq!(blockchain_data.admin_address, emulated_data.admin_address);
    assert_eq!(blockchain_data.wallet_code, emulated_data.wallet_code);
    assert_eq!(blockchain_data.content, emulated_data.content);
    Ok(())
}

async fn emulate_get_jetton_data(
    account_state: &RawFullAccountState,
    factory: &TonContractFactory,
    address: &TonAddress,
) -> anyhow::Result<JettonData> {
    let config = factory.get_config_cell_serial().await?;
    let mut c7 = assert_ok!(TvmEmulatorC7::new(address.clone(), Vec::from(config)));
    c7.with_balance(account_state.balance as u64);

    let mut emulator = assert_ok!(TvmEmulator::new(
        account_state.code.as_slice(),
        account_state.data.as_slice()
    ));
    emulator.with_c7(&c7)?;
    let result = assert_ok!(emulator.run_get_method(&"get_jetton_data".into(), &[]));

    assert!(result.exit_success());

    let stack = result.stack;
    assert_eq!(stack.len(), 5);

    let total_supply = assert_ok!(stack[0].get_biguint());
    let mintable = assert_ok!(stack[1].get_bool());
    let admin_address = assert_ok!(stack[2].get_address());
    let content = assert_ok!(MetaDataContent::parse(&assert_ok!(stack[3].get_cell())));
    let wallet_code = assert_ok!(stack[4].get_cell());

    let jetton = JettonData {
        total_supply,
        mintable,
        admin_address,
        content,
        wallet_code,
    };
    Ok(jetton)
}

#[tokio::test]
async fn test_emulator_get_wallet_address() -> anyhow::Result<()> {
    let factory = new_contract_factory(false, false).await?;

    let minter_address = TonAddress::from_str("EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR")?;
    let owner_address = TonAddress::from_str("EQB2BtXDXaQuIcMYW7JEWhHmwHfPPwa-eoCdefiAxOhU3pQg")?;
    let expected = TonAddress::from_str("EQCGY3OVLtD9KRcOsP2ldQDtuY0FMzV7wPoxjrFbayBXc23c")?;

    let contract = factory.get_contract(&minter_address);
    let state = contract.get_state().await?;

    let stack = vec![TvmStackEntry::try_from(&owner_address)?];
    let stack_slice = stack.as_slice();
    let method = "get_wallet_address";
    let method_id = method;

    let r1 = assert_ok!(state.emulate_get_method(method_id, stack_slice).await);
    let r2 = assert_ok!(state.tonlib_run_get_method(method_id, stack_slice).await);
    let r3 = assert_ok!(state.run_get_method(method, stack_slice).await);

    assert_eq!(assert_ok!(r1.stack[0].get_address()), expected);
    for res in [&r2, &r3] {
        assert_eq!(r1.stack, res.stack);
        assert_eq!(r1.gas_used, res.gas_used);
        assert_eq!(r1.vm_exit_code, res.vm_exit_code);
    }
    Ok(())
}

#[tokio::test]
async fn test_emulate_ston_router_v2() -> anyhow::Result<()> {
    common::init_logging();
    let factory = new_contract_factory(false, false).await?;

    let router_address = "EQCqX53C_Th32Xg7UyrlqF0ypmePjljxG8edlwfT-1QpG3TB".parse()?;
    let tx_id = TonTxId::from_lt_hash(
        51600010000005,
        "82218cf8373437ffeac1bf306f44d9638894c2d2b4b2bddf85ac2c571b56b2a7",
    )?;

    let contract = factory.get_contract(&router_address);
    let state = contract.get_state_by_transaction(&tx_id.into()).await?;

    let token1_addr = CellSlice::full_cell(
        CellBuilder::new()
            .store_address(&"EQC8JhkQsgAwRpe0lMsr6U11NXWjwgty22gxnRt_pSq4jDmb".parse()?)?
            .build()?,
    )?;
    let token2_addr = CellSlice::full_cell(
        CellBuilder::new()
            .store_address(&"EQB1R5vBgbJBZNVkh55XID629E2Xq9MFib3nai9QSkZ2F7X4".parse()?)?
            .build()?,
    )?;

    let call_parameters_vec = [
        ("get_router_data", vec![]),
        ("get_upgraded_pool_code", vec![]),
        ("get_router_version", vec![]),
        (
            "get_pool_address",
            vec![
                TvmStackEntry::Slice(token1_addr),
                TvmStackEntry::Slice(token2_addr),
            ],
        ),
    ];
    for call_parameters in call_parameters_vec {
        let method_id = call_parameters.0;
        let result: TvmSuccess = assert_ok!(
            state
                .emulate_get_method(method_id, call_parameters.1.as_slice())
                .await
        );

        let expected_result = state
            .tonlib_run_get_method(method_id, call_parameters.1.as_slice())
            .await?;

        log::info!(
            "Called router with method: {:?}, stack: {:?}",
            call_parameters.0,
            call_parameters.1
        );

        log::info!("METHOD:  {method_id:?}");
        log::info!("___________________Blockchain_result \n {expected_result:?} \n-------------------------------------------");
        log::info!("_____________________Emulated_result \n {result:?} \n-------------------------------------------");

        assert_eq!(result.gas_used, expected_result.gas_used);
        assert_eq!(result.missing_library, expected_result.missing_library);
        assert_eq!(result.vm_exit_code, expected_result.vm_exit_code);

        //explicitly omitted check of vm_log, as it is not returned from blockchain
        // assert_eq!(result.vm_log, expected_result.vm_log);

        for i in 0..expected_result.stack.len() {
            let (expected, actual) = (expected_result.stack[i].clone(), result.stack[i].clone());

            match (expected, actual) {
                (TvmStackEntry::Cell(e), TvmStackEntry::Cell(a)) => assert_eq!(e, a),

                (TvmStackEntry::Slice(e), TvmStackEntry::Slice(a)) => {
                    assert_eq!(e.into_cell()?, a.into_cell()?)
                }

                (TvmStackEntry::Int257(e), TvmStackEntry::Int64(a)) => assert_eq!(e, a.into()),

                (_, _) => panic!(),
            }
        }
    }
    Ok(())
}

/// Benchmark for emulator
#[ignore]
#[tokio::test]
async fn benchmark_emulate_ston_router_v2() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client_archive().await;
    let factory = TonContractFactory::builder(&client).build().await?;

    let router_address = "EQCqX53C_Th32Xg7UyrlqF0ypmePjljxG8edlwfT-1QpG3TB".parse()?;
    let tx_id = TonTxId::from_lt_hash(
        51600010000005,
        "82218cf8373437ffeac1bf306f44d9638894c2d2b4b2bddf85ac2c571b56b2a7",
    )?;

    let contract = factory.get_contract(&router_address);
    let state = contract.get_state_by_transaction(&tx_id.into()).await?;

    let token1_addr = CellSlice::full_cell(
        CellBuilder::new()
            .store_address(&"EQC8JhkQsgAwRpe0lMsr6U11NXWjwgty22gxnRt_pSq4jDmb".parse()?)?
            .build()?,
    )?;
    let token2_addr = CellSlice::full_cell(
        CellBuilder::new()
            .store_address(&"EQB1R5vBgbJBZNVkh55XID629E2Xq9MFib3nai9QSkZ2F7X4".parse()?)?
            .build()?,
    )?;

    let stack = &vec![
        TvmStackEntry::Slice(token1_addr.clone()),
        TvmStackEntry::Slice(token2_addr.clone()),
    ];
    let stack_ref: &[TvmStackEntry] = stack.as_ref();

    let code = state.get_account_state().code.clone();
    let data = state.get_account_state().data.clone();

    let code_cell = BagOfCells::parse(&code)?.single_root()?;
    let data_cell = BagOfCells::parse(&data)?.single_root()?;

    let c7 = TvmEmulatorC7::new(
        router_address.clone(),
        factory.get_config_cell_serial().await?.to_vec(),
    )?;
    let libs = factory
        .library_provider()
        .get_libs(&[code_cell, data_cell], None)
        .await?;

    let mut sums: ((Duration, Duration, Duration, Duration), Duration) = (
        (
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        ),
        Default::default(),
    );

    const MAX_ITER: usize = 10;
    for i in 0..MAX_ITER {
        let run_result: (
            (TvmSuccess, Duration, Duration, Duration, Duration),
            Duration,
        ) = unsafe {
            // Using unsafe to extend lifetime of references to method_id & stack.
            //
            // This is necessary because the compiler doesn't have a proof that these references
            // outlive spawned future.
            // But we're know it for sure since we're awaiting it. In normal async/await block
            // this would be checked by the compiler, but not when using `spawn_blocking`
            let static_method_id: TonMethodId = "get_pool_data".into();
            let static_stack: &'static [TvmStackEntry] = std::mem::transmute(stack_ref);

            let code = code.clone();
            let data = data.clone();
            let c7 = c7.clone();
            let libs = libs.0.clone();

            #[allow(clippy::let_and_return)]
            let ovetall_t = Instant::now();
            let res = tokio::task::spawn_blocking(move || {
                let code = code.as_slice();
                let data = data.as_slice();

                let t_creation = Instant::now();
                let mut emulator = TvmEmulator::new(code, data).unwrap();
                let creation_time = t_creation.elapsed();

                let t_c7 = Instant::now();
                let e = emulator.with_c7(&c7).unwrap();
                let c7_time = t_c7.elapsed();

                let t_lib = Instant::now();
                let e = e.with_libraries(libs.as_slice()).unwrap();
                let lib_time = t_lib.elapsed();

                let running_time = Instant::now();
                let run_result = e.run_get_method(&static_method_id, static_stack);
                let running_time = running_time.elapsed();
                (
                    run_result.unwrap(),
                    creation_time,
                    c7_time,
                    lib_time,
                    running_time,
                )
            })
            .await
            .map_err(|e| TonContractError::InternalError(e.to_string()))?;

            let sum_t = ovetall_t.elapsed();
            (res, sum_t)
        };

        log::info!("{} of {}: creation_time: {:?}, c7_time: {:?}, lib_time: {:?}, running_time: {:?}, overall+tokio: {:?}", i, MAX_ITER,run_result.0.1,run_result.0.2,run_result.0.3,run_result.0.4,run_result.1);

        sums.0 .0 += run_result.0 .1;
        sums.0 .1 += run_result.0 .2;
        sums.0 .2 += run_result.0 .3;
        sums.0 .3 += run_result.0 .4;
        sums.1 += run_result.1
    }

    log::info!("_________________OVERALL over {MAX_ITER}_________________");
    log::info!("creation_time: {:?}, c7_time: {:?}, lib_time: {:?}, running_time: {:?}, overall+tokio: {:?}", sums.0.0, sums.0.1,sums.0.2,sums.0.3,sums.1);

    Ok(())
}

#[tokio::test]
async fn test_get_wallet_data() -> anyhow::Result<()> {
    let factory = new_contract_factory(false, false).await?;
    // regular jetton wallet
    let addr = TonAddress::from_str("EQCS65EHXVI70mKFXJuERbAHjpy-Jh5v3hqF9mpxL_ofMMZe")?;
    let contract = factory.get_contract(&addr);
    let wallet_data = assert_ok!(contract.get_wallet_data().await);
    assert_eq!(
        wallet_data.owner_address,
        "EQCS65EGyiApUTLOYXDs4jOLoQNCE0o8oNnkmfIcm0iX5AmW".parse()?
    );
    assert_eq!(
        wallet_data.master_address,
        "EQB7OmvtVzILIJzkkeqNYeb5mr-sVAqgacvKpgux2JbX7Dmg".parse()?
    );

    // weird jetton wallet with 5 elements in stack
    let addr = TonAddress::from_str("EQBncdA9V_79UPebRsvRO5xFgTT6lcnHW6u5BP1r6IS4T8jn")?;
    let contract = factory.get_contract(&addr);
    let wallet_data = assert_ok!(contract.get_wallet_data().await);
    assert_eq!(
        wallet_data.owner_address,
        "EQCDT9dCT52pdfsLNW0e6qP5T3cgq7M4Ug72zkGYgP17tsWD".parse()?
    );
    assert_eq!(
        wallet_data.master_address,
        "EQBHi6foQfVfj_moRtubwuFdmDlOM1pNFYBio9clh089NYYs".parse()?
    );

    Ok(())
}
