mod common;

mod contract_emulator_tests {
    use std::ops::Neg;

    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use lazy_static::lazy_static;
    use num_bigint::{BigInt, BigUint};
    use tokio::{self};
    use tokio_test::assert_ok;
    use tonlib::address::TonAddress;
    use tonlib::cell::{BagOfCells, CellBuilder, CellSlice};
    use tonlib::client::TonClientInterface;
    use tonlib::contract::{
        JettonData, JettonMasterContract, TonContractFactory, TonContractInterface,
    };
    use tonlib::emulator::{TvmEmulator, TvmEmulatorC7Builder};
    use tonlib::message::JettonTransferMessage;
    use tonlib::meta::MetaDataContent;
    use tonlib::types::TvmStackEntry;

    use crate::common;

    lazy_static! {
        pub static ref TEST_CONTRACT_CODE: Vec<u8> = hex::decode(
            "b5ee9c7241022101000739000114ff00f4a413f4bcf2c80b0102016205020201\
            2004030009bdb05c1ffc0007bfe45d440202c912060103b0f00704f62082300d\
            e0b6b3a7640000ba9b30823025b946ebc0b36173e08200c354218235c702bd3a\
            30fc0000be228238070c1cc73b00c80000bbb0f2f420c1008e1282300de0b6b3\
            a76400005202a3f04712a984e020821b782dace9d9aa18bee30f01a764823805\
            6bc75e2d6310000021822056bc75e2d631aa18bee3002111100f0802f4822056\
            bc75e2d631aa17be8e2701822056bc75e2d631aa17a101824adf0ab5a80a22c6\
            1ab5a7008238056bc75e2d63100000a984de21822056bc75e2d631aa16be8e26\
            01822056bc75e2d631aa16a10182403f1fce3da636ea5cf8508238056bc75e2d\
            63100000a984de21823815af1d78b58c400000bee300210e0902f482380ad78e\
            bc5ac6200000be8e260182380ad78ebc5ac6200000a1018238280e60114edb80\
            5d038238056bc75e2d63100000a984de218238056bc75e2d63100000be8e2601\
            8238056bc75e2d63100000a10182380ebc5fb417461211108238056bc75e2d63\
            100000a984de218232b5e3af16b1880000bee300210d0a01ec82315af1d78b58\
            c40000be8e250182315af1d78b58c40000a101823806f5f17757889379378238\
            056bc75e2d63100000a984de218238056bc75e2d6310000021a0511382380ad7\
            8ebc5ac6200000a98466a0511382381043561a8829300000a98466a051138238\
            15af1d78b58c400000a98466a051130b01ea82381b1ae4d6e2ef500000a98466\
            a0511382382086ac351052600000a98466a05113823825f273933db5700000a9\
            8466a05113822056bc75e2d631aa16a98466a05113823830ca024f987b900000\
            a98466a0511382383635c9adc5dea00000a98466a0511382383ba1910bf341b0\
            0000a98466a0030c00428238410d586a20a4c00000a98412a08238056bc75e2d\
            63100000a984018064a984004a018232b5e3af16b1880000a101823808f00f76\
            0a4b2db55d8238056bc75e2d63100000a984004c01823815af1d78b58c400000\
            a101823927fa27722cc06cc5e28238056bc75e2d63100000a984003830822056\
            bc75e2d631aa18a18261855144814a7ff805980ff0084000005020821b782dac\
            e9d9aa17be8e18821b782dace9d9aa17a182501425982cf597cd205cef738091\
            71e20042821b782dace9d9aa18a18288195e54c5dd42177f53a27172fa9ec630\
            262827aa230201201e130103aee01401f62082300de0b6b3a7640000b98e1182\
            300de0b6b3a76400005202a984f03ba3e0702182b05803bcc5cb9634ba4cfb22\
            13f784019318ed4dcb6017880faa35be8e23308288195e54c5dd42177f53a271\
            72fa9ec630262827aa23a904821b782dace9d9aa18de2182708bcc0026baae9e\
            45e470190267a230cfaa18be1502ea8e1c0182501425982cf597cd205cef7380\
            a90401821b782dace9d9aa17a0dea76401a764208261855144814a7ff805980f\
            f0084000be8e2a8238056bc75e2d631000008261855144814a7ff805980ff008\
            4000a98401822056bc75e2d631aa18a001de20824adf0ab5a80a22c61ab5a700\
            bee300201d1602f882403f1fce3da636ea5cf850be8e268238056bc75e2d6310\
            000082403f1fce3da636ea5cf850a98401822056bc75e2d631aa16a001de2082\
            3927fa27722cc06cc5e2be8e268238056bc75e2d63100000823927fa27722cc0\
            6cc5e2a98401823815af1d78b58c400000a001de208238280e60114edb805d03\
            bee300201c1702f482380ebc5fb41746121110be8e268238056bc75e2d631000\
            0082380ebc5fb41746121110a984018238056bc75e2d63100000a001de208238\
            08f00f760a4b2db55dbe8e258238056bc75e2d63100000823808f00f760a4b2d\
            b55da984018232b5e3af16b1880000a001de20823806f5f1775788937937bee3\
            00201b1801ec823806248f33704b286603be8e258238056bc75e2d6310000082\
            3806248f33704b286603a984018230ad78ebc5ac620000a001de20823805c548\
            670b9510e7acbe8e258238056bc75e2d63100000823805c548670b9510e7aca9\
            8401823056bc75e2d6310000a001de208238056bc75e2d63100000a11901fe82\
            38056bc75e2d631000005122a012a98453008238056bc75e2d63100000a9845c\
            8238056bc75e2d63100000a9842073a90413a051218238056bc75e2d63100000\
            a9842075a90413a051218238056bc75e2d63100000a9842077a90413a0512182\
            38056bc75e2d63100000a9842079a90413a0598238056bc75e2d631000001a00\
            1ca984800ba904a0aa00a08064a904004a8238056bc75e2d63100000823806f5\
            f1775788937937a9840182315af1d78b58c40000a001004c8238056bc75e2d63\
            1000008238280e60114edb805d03a9840182380ad78ebc5ac6200000a001004e\
            8238056bc75e2d63100000824adf0ab5a80a22c61ab5a700a98401822056bc75\
            e2d631aa17a001020120201f0063a46410e0804c45896c678b00d180ef381038\
            c70a023d5486531812d40950025503815210e0002298731819d5016780e4e840\
            0005d17c126e3e0998",
        )
        .ok()
        .unwrap();
        pub static ref BAD_CONTRACT_CODE: Vec<u8> = vec![];
        pub static ref BAD_CONTRACT_CODE_CELL: Vec<u8> = hex::decode(
            "b5ee9c7241022101000739000114ff00f4a413f4bcf2c80b0102016205020201\
        2004030009bdb05c1ffc0007bfe45d440202c912060103b0f00704f62082300d\
        e0b6b3a7640000ba9b30823025b946ebc0b36173e08200c354218235c702bd3a\
        30fc0000be228238070c1cc73b00c80000bbb0f2f420c1008e1282300de0b6b3\
        a76400005202a3f04712a984e020821b782dace9d9aa18bee30f01a764823805\
        6bc75e2d6310000021822056bc75e2d631aa18bee3002111100f0802f4822056\
        bc75e2d631aa17be8e2701822056bc75e2d631aa17a101824adf0ab5a80a22c6\
        1ab5a7008238056bc75e2d63100000a984de21822056bc75e2d631aa16be8e26\
        01822056bc75e2d631aa16a10182403f1fce3da636ea5cf8508238056bc75e2d\
        63100000a984de21823815af1d78b58c400000bee300210e0902f482380ad78e\
        bc5ac6200000be8e260182380ad78ebc5ac6200000",
        )
        .ok()
        .unwrap();
        pub static ref EMPTY: Vec<u8> = hex::decode("",).ok().unwrap();
        pub static ref TEST_CONTRACT_DATA: Vec<u8> =
            BagOfCells::from_root(CellBuilder::new().build().ok().unwrap())
                .serialize(false)
                .ok()
                .unwrap();
        pub static ref EMPTY_STACK: Vec<TvmStackEntry> = vec![];
    }

    #[tokio::test]
    async fn test_emulator_get_nan() {
        common::init_logging();
        let mut emulator = assert_ok!(TvmEmulator::new(&TEST_CONTRACT_CODE, &TEST_CONTRACT_DATA));
        let emulator_result =
            assert_ok!(emulator.run_get_method(&"get_nan".into(), EMPTY_STACK.as_slice()));

        assert_eq!(emulator_result.stack.len(), 1);
        assert_eq!(emulator_result.stack[0], TvmStackEntry::Nan);
    }

    #[tokio::test]
    async fn test_emulator_empty_contract_code() {
        common::init_logging();
        // empty code  empty data
        let emulator_result = TvmEmulator::new(&EMPTY, &EMPTY);
        log::info!("{:?}", emulator_result);
        assert!(emulator_result.is_err());

        // bad code empty data
        let emulator_result = TvmEmulator::new(&BAD_CONTRACT_CODE, &TEST_CONTRACT_DATA);
        log::info!("{:?}", emulator_result);
        assert!(emulator_result.is_err());

        // bad code cell empty data
        let emulator_result = TvmEmulator::new(&BAD_CONTRACT_CODE_CELL, &TEST_CONTRACT_DATA);
        log::info!("{:?}", emulator_result);
        assert!(emulator_result.is_err());

        // Ok code cell empty data
        let emulator_result = TvmEmulator::new(&TEST_CONTRACT_CODE, &EMPTY);
        log::info!("{:?}", emulator_result);
        assert!(emulator_result.is_err());
    }

    #[tokio::test]
    async fn test_emulator_bigint_multiply() {
        common::init_logging();
        bigint_multiplier(&BigInt::from(1), &BigInt::from(0x1234567890ABCDEFu64));
        bigint_multiplier(&BigInt::from(1), &BigInt::from(0x1234567890ABCDEFu64).neg());
        bigint_multiplier(
            &BigInt::from(10_000_000_000_i64),
            &BigInt::from(0x1234567890ABCDEFu64),
        );
    }

    fn bigint_multiplier(val1: &BigInt, val2: &BigInt) {
        let expected = val1 * val2;
        log::info!("Testing: {} = {} * {}", expected, val1, val2);
        let mut emulator = assert_ok!(TvmEmulator::new(&TEST_CONTRACT_CODE, &TEST_CONTRACT_DATA));
        assert_ok!(emulator.set_debug_enable());
        let stack = vec![val1.clone().into(), val2.clone().into()];
        let emulator_result =
            assert_ok!(emulator.run_get_method(&"get_val".into(), stack.as_slice()));
        assert_eq!(emulator_result.vm_exit_code, 0);
        assert_eq!(emulator_result.stack.len(), 1);
        log::info!("{:?}", emulator_result.stack);
        let result = assert_ok!(emulator_result.stack[0].get_bigint());
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_emulator_i64_multiply() {
        common::init_logging();
        i64_multiplier(1, 0x1234567890ABCDEFi64);
        i64_multiplier(1, -0x1234567890ABCDEFi64);
        i64_multiplier(-1, 0x1234567890ABCDEFi64);
        i64_multiplier(10_000_000_000_i64, 0x1234567890ABCDEFi64);
    }

    fn i64_multiplier(val1: i64, val2: i64) {
        let expected = BigInt::from(val1) * BigInt::from(val2);
        log::info!("Testing: {} = {} * {}", expected, val1, val2);
        let mut emulator = assert_ok!(TvmEmulator::new(&TEST_CONTRACT_CODE, &TEST_CONTRACT_DATA));
        assert_ok!(emulator.set_debug_enable());
        let stack = vec![val1.into(), val2.into()];
        let emulator_result =
            assert_ok!(emulator.run_get_method(&"get_val".into(), stack.as_slice()));
        assert_eq!(emulator_result.vm_exit_code, 0);

        assert_eq!(emulator_result.stack.len(), 1);
        log::info!("{:?}", emulator_result.stack);
        let result = assert_ok!(emulator_result.stack[0].get_bigint());
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_emulator_get_jetton_data() {
        common::init_logging();
        let client = common::new_mainnet_client().await;

        let address = assert_ok!(TonAddress::from_base64_url(
            "EQDCJL0iQHofcBBvFBHdVG233Ri2V4kCNFgfRT-gqAd3Oc86"
        )); //jetton master
        let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
        let contract = factory.get_contract(&address);
        let account_state = assert_ok!(contract.get_account_state().await);

        let code = &account_state.code;
        log::info!("code cell: {}", STANDARD.encode(code.as_slice()));
        let data = &account_state.data;

        let blockchain_data: JettonData = assert_ok!(contract.get_jetton_data().await);
        let emulated_data = emulate_get_jetton_data(code, data);

        log::info!("{:?}\n {:?} ", blockchain_data, emulated_data);

        assert_eq!(blockchain_data.total_supply, emulated_data.total_supply);
        assert_eq!(blockchain_data.mintable, emulated_data.mintable);
        assert_eq!(blockchain_data.admin_address, emulated_data.admin_address);
        assert_eq!(blockchain_data.wallet_code, emulated_data.wallet_code);
        assert_eq!(blockchain_data.content, emulated_data.content);
    }

    #[tokio::test]
    async fn test_emulator_get_jetton_data_long_total_supply() {
        common::init_logging();
        let client = common::new_mainnet_client().await;

        let address = assert_ok!(TonAddress::from_base64_url(
            "EQAW42HutyDem98Be1f27PoXobghh81umTQ-cGgaKVmRLS7-"
        )); //jetton master
        let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
        let contract = factory.get_contract(&address);
        let account_state = assert_ok!(contract.get_account_state().await);

        let code = &account_state.code;
        log::info!("code cell: {}", STANDARD.encode(code.as_slice()));
        let data = &account_state.data;
        log::info!("data cell: {}", STANDARD.encode(data.as_slice()));
        let blockchain_data = assert_ok!(contract.get_jetton_data().await);
        let emulated_data = emulate_get_jetton_data(code, data);

        log::info!("{:?}\n {:?} ", blockchain_data, emulated_data);

        assert_eq!(blockchain_data.total_supply, emulated_data.total_supply);
        assert_eq!(blockchain_data.mintable, emulated_data.mintable);
        assert_eq!(blockchain_data.admin_address, emulated_data.admin_address);
        assert_eq!(blockchain_data.wallet_code, emulated_data.wallet_code);
        assert_eq!(blockchain_data.content, emulated_data.content);
    }

    fn emulate_get_jetton_data(code: &[u8], data: &[u8]) -> JettonData {
        const JETTON_DATA_STACK_ELEMENTS: usize = 5;
        let method = "get_jetton_data";

        let emulator_result = assert_ok!(assert_ok!(TvmEmulator::new(code, data))
            .run_get_method(&method.into(), EMPTY_STACK.as_slice()));

        let stack = emulator_result.stack;

        assert_eq!(stack.len(), JETTON_DATA_STACK_ELEMENTS);

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

    #[allow(dead_code)]
    #[tokio::test]
    async fn test_get_jetton_wallet() {
        common::init_logging();
        let client = common::new_mainnet_client().await;

        let minter_address = assert_ok!("EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR".parse()); //jetton master
        let owner_address = assert_ok!("EQB2BtXDXaQuIcMYW7JEWhHmwHfPPwa-eoCdefiAxOhU3pQg".parse());
        let expected: TonAddress =
            assert_ok!("EQCGY3OVLtD9KRcOsP2ldQDtuY0FMzV7wPoxjrFbayBXc23c".parse());
        let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
        let contract = factory.get_contract(&minter_address);
        let state = assert_ok!(contract.get_account_state().await);
        let info = assert_ok!(client.get_config_all(0).await);
        let config_data = info.config.bytes;

        log::info!("code cell: {}", STANDARD.encode(state.code.as_slice()));
        log::info!("data cell: {}", STANDARD.encode(state.data.as_slice()));
        let emulated_result = emulate_get_wallet_address(
            &state.code,
            &state.data,
            &minter_address,
            &owner_address,
            &config_data,
        );
        assert_eq!(emulated_result, expected);
        let blockchain_result = assert_ok!(contract.get_wallet_address(&owner_address).await);
        assert_eq!(blockchain_result, expected);
    }

    fn emulate_get_wallet_address(
        code: &[u8],
        data: &[u8],
        self_address: &TonAddress,
        owner_address: &TonAddress,
        config_data: &[u8],
    ) -> TonAddress {
        let mut emulator = assert_ok!(TvmEmulator::new(code, data));

        let tvm_emulator_c7 = TvmEmulatorC7Builder::new(self_address, config_data, 0).build();

        assert_ok!(emulator.set_c7(&tvm_emulator_c7));
        let stack: Vec<TvmStackEntry> = vec![assert_ok!(owner_address.try_into())];
        let emulator_result =
            assert_ok!(emulator.run_get_method(&"get_wallet_address".into(), stack.as_slice()));
        assert!(emulator_result.exit_success());

        assert_eq!(emulator_result.stack.len(), 1);
        assert_ok!(emulator_result.stack[0].get_address())
    }

    #[tokio::test]
    async fn test_address_in_stack() {
        common::init_logging();
        let client = common::new_mainnet_client().await;

        let pool_address = assert_ok!("EQDtZHOtVWaf9UIU6rmjLPNLTGxNLNogvK5xUZlMRgZwQ4Gt".parse());
        let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
        let account_state = assert_ok!(factory.get_latest_account_state(&pool_address).await);
        let code = account_state.code.as_slice();
        let data = account_state.data.as_slice();
        let (addr1, addr2) = emulate_get_pool_data(code, data);
        log::info!("Addr1: {}, Addr2: {}", addr1, addr2);
        let amount = BigUint::from(100_500_000u32);
        let emulated_result = emulate_get_expected_outputs(code, data, &amount, &addr1);
        log::info!("Emulated result: {}", emulated_result);
        let addr_cell = assert_ok!(assert_ok!(CellBuilder::new().store_address(&addr1)).build());
        let stack = vec![
            TvmStackEntry::Int257(BigInt::from(amount)),
            TvmStackEntry::Slice(assert_ok!(CellSlice::full_cell(addr_cell))),
        ];
        let run_result = assert_ok!(
            assert_ok!(
                factory
                    .get_contract(&pool_address)
                    .get_state_by_transaction(&account_state.last_transaction_id)
                    .await
            )
            .run_get_method("get_expected_outputs", stack)
            .await
        );
        assert!(run_result.vm_exit_code == 0 || run_result.vm_exit_code == 1);
        assert_eq!(run_result.stack.len(), 3);
        let state_result = assert_ok!(run_result.stack[0].get_biguint());
        log::info!("Blockchain result: {}", state_result);
        assert_eq!(emulated_result, state_result);
    }

    fn emulate_get_pool_data(code: &[u8], data: &[u8]) -> (TonAddress, TonAddress) {
        let mut emulator = assert_ok!(TvmEmulator::new(code, data));
        let emulator_result =
            assert_ok!(emulator.run_get_method(&"get_pool_data".into(), vec![].as_slice()));
        assert!(emulator_result.exit_success());

        assert_eq!(emulator_result.stack.len(), 10);
        let addr1 = assert_ok!(emulator_result.stack[2].get_address());
        let addr2 = assert_ok!(emulator_result.stack[3].get_address());
        (addr1, addr2)
    }

    fn emulate_get_expected_outputs(
        code: &[u8],
        data: &[u8],
        amount: &BigUint,
        token_wallet: &TonAddress,
    ) -> BigUint {
        let mut emulator = assert_ok!(TvmEmulator::new(code, data));
        let stack = vec![amount.clone().into(), assert_ok!(token_wallet.try_into())];
        let emulator_result =
            assert_ok!(emulator.run_get_method(&"get_expected_outputs".into(), stack.as_slice()));
        assert!(emulator_result.exit_success());
        assert_eq!(emulator_result.stack.len(), 3);
        assert_ok!(emulator_result.stack[0].get_biguint())
    }

    #[tokio::test]
    async fn emulate_external_transfer_message() {
        common::init_logging();
        let client = common::new_mainnet_client().await;

        let address = assert_ok!(TonAddress::from_base64_url(
            "EQAW42HutyDem98Be1f27PoXobghh81umTQ-cGgaKVmRLS7-"
        )); //jetton master
        let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
        let contract = factory.get_contract(&address);
        let account_state = assert_ok!(contract.get_account_state().await);

        let code = &account_state.code;
        log::info!("code cell: {}", STANDARD.encode(code.as_slice()));
        let data = &account_state.data;
        log::info!("data cell: {}", STANDARD.encode(data.as_slice()));

        let address = assert_ok!(TonAddress::from_base64_url(
            "Ef8CmPZLxWB-9ypeGdGhEqA6ZNLBFUwnqXPH2eUQd_MzbGh_"
        ));
        let msg = assert_ok!(JettonTransferMessage::new(&address, &BigUint::from(1u32)).build());

        let mut emulator = assert_ok!(TvmEmulator::new(code, data));
        let r = assert_ok!(emulator.send_external_message(msg));
        log::info!("RES: {:?}", r);
        assert_eq!(r.gas_used, 270);
        assert_eq!(r.vm_exit_code, 11);
    }

    #[tokio::test]
    async fn emulate_internal_transfer_message() -> anyhow::Result<()> {
        common::init_logging();
        let client = common::new_mainnet_client().await;

        let address = assert_ok!(TonAddress::from_base64_url(
            "EQAW42HutyDem98Be1f27PoXobghh81umTQ-cGgaKVmRLS7-"
        )); //jetton master
        let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
        let contract = factory.get_contract(&address);
        let account_state = assert_ok!(contract.get_account_state().await);

        let code = &account_state.code;
        log::info!("code cell: {}", STANDARD.encode(code.as_slice()));
        let data = &account_state.data;
        log::info!("data cell: {}", STANDARD.encode(data.as_slice()));

        let address = assert_ok!(TonAddress::from_base64_url(
            "Ef8CmPZLxWB-9ypeGdGhEqA6ZNLBFUwnqXPH2eUQd_MzbGh_"
        ));
        let msg = assert_ok!(JettonTransferMessage::new(&address, &BigUint::from(1u32)).build());

        let mut emulator = assert_ok!(TvmEmulator::new(code, data));
        let r = assert_ok!(emulator.send_internal_message(msg, 300));
        log::info!("RES: {:?}", r);
        assert_eq!(r.gas_used, 2779);
        assert_eq!(r.vm_exit_code, 65535);
        Ok(())
    }

    #[tokio::test]
    async fn test_convert_lib_addr() {
        common::init_logging();
        let hex_addr = TonAddress::from_hex_str(
            "4F0171272C215B8BF8FEEAC46A857688A4B65F4FE61F8228631F627D0EDA9D00",
        );

        log::info!("addr {:?}", hex_addr);
    }

    #[tokio::test]
    async fn test_get_lib_cells() {
        common::init_logging();
        let client = common::new_mainnet_client().await;
        let factory = assert_ok!(TonContractFactory::builder(&client).build().await);

        let minter_lib_address = assert_ok!(TonAddress::from_base64_url(
            "Ef8CmPZLxWB-9ypeGdGhEqA6ZNLBFUwnqXPH2eUQd_MzbGh_"
        ));

        let minter_lib = assert_ok!(
            factory
                .get_contract(&minter_lib_address)
                .get_account_state()
                .await
        );

        log::info! {"{:?}", minter_lib};
    }

    #[cfg(feature = "emulate_get_method")]
    #[tokio::test]
    async fn test_emulator_contract_with_library() {
        common::init_logging();
        let client = common::new_mainnet_client().await;

        let address = assert_ok!(TonAddress::from_base64_url(
            "EQDqVNU7Jaf85MhIba1lup0F7Mr3rGigDV8RxMS62RtFr1w8"
        )); //jetton master

        let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
        let contract = factory.get_contract(&address);
        let blockchain_data = assert_ok!(contract.get_jetton_data().await);

        let factory_with_library_loader =
            assert_ok!(TonContractFactory::builder(&client).build().await);
        let contract_with_library_loader = factory_with_library_loader.get_contract(&address);
        let emulated_data = assert_ok!(contract_with_library_loader.get_jetton_data().await);

        log::info! {"Blockchain Jetton data: {:?}", blockchain_data};
        log::info! {"Emulated Jetton data: {:?}", blockchain_data};

        assert_eq!(blockchain_data, emulated_data);
    }
}
