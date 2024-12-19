mod common;
use tokio_test::assert_ok;
use tonlib_client::contract::{
    JettonData, JettonMasterContract, TonContractFactory, TonContractInterface,
};
use tonlib_client::emulator::c7_register::TvmEmulatorC7;
use tonlib_client::emulator::tvm_emulator::TvmEmulator;
use tonlib_client::meta::MetaDataContent;
use tonlib_client::tl::RawFullAccountState;
use tonlib_client::types::TvmStackEntry;
use tonlib_core::cell::{CellBuilder, CellSlice};
use tonlib_core::{TonAddress, TonTxId};

#[tokio::test]
async fn test_emulator_get_jetton_data() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;

    let address = assert_ok!(TonAddress::from_base64_url(
        "EQDCJL0iQHofcBBvFBHdVG233Ri2V4kCNFgfRT-gqAd3Oc86"
    )); //jetton master
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&address);
    let state = assert_ok!(contract.get_account_state().await);

    let emulated_data = emulate_get_jetton_data(&state, &factory, &address).await?;
    let blockchain_data = assert_ok!(contract.get_jetton_data().await);

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
    let c7 = assert_ok!(TvmEmulatorC7::new(address.clone(), Vec::from(config)))
        .with_balance(account_state.balance as u64);

    let mut emulator = assert_ok!(TvmEmulator::new(
        account_state.code.as_slice(),
        account_state.data.as_slice()
    ))
    .with_c7(&c7)?;
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
async fn test_emulator_get_wallet_address() {
    common::init_logging();
    let client = common::new_mainnet_client().await;

    let minter_address = assert_ok!("EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR".parse());
    let owner_address = &assert_ok!(TonAddress::from_base64_url(
        "EQB2BtXDXaQuIcMYW7JEWhHmwHfPPwa-eoCdefiAxOhU3pQg"
    ));
    let expected: TonAddress =
        assert_ok!("EQCGY3OVLtD9KRcOsP2ldQDtuY0FMzV7wPoxjrFbayBXc23c".parse());

    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&minter_address);
    let state = assert_ok!(contract.get_state().await);

    let stack = vec![assert_ok!(owner_address.try_into())];
    let method = "get_wallet_address";
    let method_id = method;

    let r1 = assert_ok!(state.emulate_get_method(method_id, stack.as_slice()).await);
    let r2 = assert_ok!(
        state
            .tonlib_run_get_method(method_id, stack.as_slice())
            .await
    );
    let r3 = assert_ok!(state.run_get_method(method, stack.as_slice()).await);

    assert_eq!(assert_ok!(r1.stack[0].get_address()), expected);
    assert_eq!(r1.stack, r2.stack);
    assert_eq!(r1.stack, r3.stack);

    assert_eq!(r1.gas_used, r2.gas_used);
    assert_eq!(r1.gas_used, r3.gas_used);
    assert_eq!(r1.vm_exit_code, r2.vm_exit_code);
    assert_eq!(r1.vm_exit_code, r3.vm_exit_code);
}

#[tokio::test]
async fn test_emulate_ston_router_v2() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
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
        let result: tonlib_client::types::TvmSuccess = assert_ok!(
            state
                .emulate_get_method(method_id, call_parameters.1.as_slice())
                .await
        );

        let expected_result = state
            .tonlib_run_get_method(method_id, call_parameters.1.as_slice())
            .await
            .unwrap();

        log::info!(
            "Called router with method: {:?}, stack: {:?}",
            call_parameters.0,
            call_parameters.1
        );

        log::info!("METHOD:  {:?}", method_id);
        log::info!("___________________Blockchain_result \n {:?} \n-------------------------------------------", expected_result);
        log::info!("_____________________Emulated_result \n {:?} \n-------------------------------------------", result);

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
                    assert_eq!(e.into_cell().unwrap(), a.into_cell().unwrap())
                }

                (TvmStackEntry::Int257(e), TvmStackEntry::Int64(a)) => assert_eq!(e, a.into()),

                (_, _) => panic!(),
            }
        }
    }
    Ok(())
}
