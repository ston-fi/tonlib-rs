mod common;
mod contract_emulator_tests {
    use tokio_test::assert_ok;
    use tonlib::address::TonAddress;
    use tonlib::contract::{
        JettonData, JettonMasterContract, TonContractFactory, TonContractInterface,
    };
    use tonlib::emulator::{TvmEmulator, TvmEmulatorC7Builder};
    use tonlib::meta::MetaDataContent;
    use tonlib::tl::RawFullAccountState;

    use crate::common;

    #[tokio::test]
    async fn test_emulator_get_jetton_data() {
        common::init_logging();
        let client = common::new_mainnet_client().await;

        let address = assert_ok!(TonAddress::from_base64_url(
            "EQDCJL0iQHofcBBvFBHdVG233Ri2V4kCNFgfRT-gqAd3Oc86"
        )); //jetton master
        let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
        let contract = factory.get_contract(&address);
        let state = assert_ok!(contract.get_account_state().await);

        let emulated_data = emulate_get_jetton_data(&state, &factory, &address).await;
        let blockchain_data = assert_ok!(contract.get_jetton_data().await);

        assert_eq!(blockchain_data.total_supply, emulated_data.total_supply);
        assert_eq!(blockchain_data.mintable, emulated_data.mintable);
        assert_eq!(blockchain_data.admin_address, emulated_data.admin_address);
        assert_eq!(blockchain_data.wallet_code, emulated_data.wallet_code);
        assert_eq!(blockchain_data.content, emulated_data.content);
    }

    async fn emulate_get_jetton_data(
        account_state: &RawFullAccountState,
        factory: &TonContractFactory,
        address: &TonAddress,
    ) -> JettonData {
        let mut emulator = assert_ok!(TvmEmulator::new(
            account_state.code.as_slice(),
            account_state.data.as_slice()
        ));
        let config = assert_ok!(factory.get_config_cell_serial().await);
        let c7 = TvmEmulatorC7Builder::new(address, config, account_state.balance as u64).build();
        assert_ok!(emulator.set_c7(&c7));
        let result = assert_ok!(emulator.run_get_method(&"get_jetton_data".into(), &[]));

        assert!(result.exit_success());

        let stack = result.stack;
        assert_eq!(stack.len(), 5);

        let total_supply = assert_ok!(stack[0].get_biguint());
        let mintable = assert_ok!(stack[1].get_bool());
        let admin_address = assert_ok!(stack[2].get_address());
        let content = assert_ok!(MetaDataContent::parse(&assert_ok!(stack[3].get_cell())));
        let wallet_code = assert_ok!(stack[4].get_cell());

        JettonData {
            total_supply,
            mintable,
            admin_address,
            content,
            wallet_code,
        }
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
}
