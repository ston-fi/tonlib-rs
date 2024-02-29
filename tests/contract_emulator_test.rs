mod common;
#[cfg(feature = "emulate_get_method")]
mod contract_emulator_tests {
    use anyhow::bail;
    use tonlib::address::TonAddress;
    use tonlib::contract::{
        JettonData, JettonMasterContract, TonContractFactory, TonContractInterface,
    };
    use tonlib::emulator::{TvmEmulator, TvmEmulatorC7Builder};
    use tonlib::meta::MetaDataContent;
    use tonlib::tl::RawFullAccountState;

    use crate::common;

    #[tokio::test]
    async fn test_emulator_get_jetton_data() -> anyhow::Result<()> {
        common::init_logging();
        let client = common::new_mainnet_client().await?;

        let address =
            TonAddress::from_base64_url("EQDCJL0iQHofcBBvFBHdVG233Ri2V4kCNFgfRT-gqAd3Oc86")?; //jetton master
        let factory = TonContractFactory::builder(&client).build().await?;
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
        let mut emulator =
            TvmEmulator::new(account_state.code.as_slice(), account_state.data.as_slice())?;
        let config = factory.get_config_cell_serial().await?;
        let c7 = TvmEmulatorC7Builder::new(address, config, account_state.balance as u64).build();
        emulator.set_c7(&c7)?;
        let result = emulator.run_get_method(&"get_jetton_data".into(), &vec![])?;

        if !result.exit_success() {
            bail!("Unsuccessful exit: {:?}", result)
        }
        let stack = result.stack;
        if stack.len() == 5 {
            let total_supply = stack[0].get_biguint()?;
            let mintable = stack[1].get_bool()?;
            let admin_address = stack[2].get_address()?;
            let content = MetaDataContent::parse(&stack[3].get_cell()?)?;
            let wallet_code = stack[4].get_cell()?;
            let data = JettonData {
                total_supply,
                mintable,
                admin_address,
                content,
                wallet_code,
            };
            Ok(data)
        } else {
            bail!("Expected 5 elements, got {}", stack.len())
        }
    }

    #[cfg(feature = "emulate_get_method")]
    #[tokio::test]
    async fn test_emulator_get_wallet_address() -> anyhow::Result<()> {
        common::init_logging();
        let client = common::new_mainnet_client().await?;

        let minter_address = "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR".parse()?;
        let owner_address =
            &TonAddress::from_base64_url("EQB2BtXDXaQuIcMYW7JEWhHmwHfPPwa-eoCdefiAxOhU3pQg")?;
        let expected: TonAddress = "EQCGY3OVLtD9KRcOsP2ldQDtuY0FMzV7wPoxjrFbayBXc23c".parse()?;

        let factory = TonContractFactory::builder(&client).build().await?;
        let contract = factory.get_contract(&minter_address);
        let state = contract.get_state().await?;

        let stack = vec![owner_address.try_into()?];
        let method = "get_wallet_address";
        let method_id = method;

        let r1 = state
            .emulate_get_method(method_id, stack.as_slice())
            .await?;
        let r2 = state
            .tonlib_run_get_method(method_id, stack.as_slice())
            .await?;
        let r3 = state.run_get_method(method, stack.as_slice()).await?;

        assert_eq!(r1.stack[0].get_address()?, expected);
        assert_eq!(r1.stack, r2.stack);
        assert_eq!(r1.stack, r3.stack);

        assert_eq!(r1.gas_used, r2.gas_used);
        assert_eq!(r1.gas_used, r3.gas_used);
        assert_eq!(r1.vm_exit_code, r2.vm_exit_code);
        assert_eq!(r1.vm_exit_code, r3.vm_exit_code);

        Ok(())
    }
}
