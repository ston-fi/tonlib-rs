use num_bigint::BigUint;
use tokio_test::assert_ok;
use tonlib::address::TonAddress;
use tonlib::cell::{
    key_extractor_u8, value_extractor_uint, CellSlice, DictLoader, GenericDictLoader, TonCellError,
};
use tonlib::contract::{TonContractFactory, TonContractInterface};

mod common;
#[derive(Debug)]
pub struct FarmDataAccrued {
    pub deposited_nanorewards: BigUint,
    pub accrued_per_unit_nanorewards: BigUint,
    pub accrued_fee_nanorewards: BigUint,
    pub claimed_nanorewards: BigUint,
    pub claimed_fee_nanorewards: BigUint,
    pub accrued_nanorewards: BigUint,
    pub last_update_time: u64,
}

#[derive(Debug)]
pub struct FarmDataParameters {
    pub admin_fee: u16,
    pub nanorewards_per_24h: BigUint,
    pub unrestricted_deposit_rewards: bool,
    pub reward_token_wallet: TonAddress,
    pub can_change_fee: bool,
    pub status: u8,
}

struct FarmDataAccruedLoader {}

impl DictLoader<u8, FarmDataParameters> for FarmDataParametersLoader {
    fn extract_key(&self, key: &[u8]) -> Result<u8, TonCellError> {
        key_extractor_u8(self.key_bit_len(), key)
    }

    fn extract_value(&self, cell_slice: &CellSlice) -> Result<FarmDataParameters, TonCellError> {
        let data_cell = assert_ok!(cell_slice.reference(0));
        let mut parser = data_cell.parser();
        let admin_fee = assert_ok!(parser.load_u16(16));
        let nanorewards_per_24h = assert_ok!(parser.load_uint(150));
        let unrestricted_deposit_rewards = assert_ok!(parser.load_bit());
        let reward_token_wallet = assert_ok!(parser.load_address());
        let can_change_fee = assert_ok!(parser.load_bit());
        let status = assert_ok!(parser.load_u8(8));
        let result = FarmDataParameters {
            admin_fee,
            nanorewards_per_24h,
            unrestricted_deposit_rewards,
            reward_token_wallet,
            can_change_fee,
            status,
        };
        Ok(result)
    }

    fn key_bit_len(&self) -> usize {
        8
    }
}

struct FarmDataParametersLoader {}

impl DictLoader<u8, FarmDataAccrued> for FarmDataAccruedLoader {
    fn extract_key(&self, key: &[u8]) -> Result<u8, TonCellError> {
        key_extractor_u8(self.key_bit_len(), key)
    }

    fn extract_value(&self, cell_slice: &CellSlice) -> Result<FarmDataAccrued, TonCellError> {
        let data_cell = assert_ok!(cell_slice.reference(0));
        let mut parser = data_cell.parser();
        let deposited_nanorewards = assert_ok!(parser.load_uint(150));
        let accrued_per_unit_nanorewards = assert_ok!(parser.load_uint(150));
        let accrued_fee_nanorewards = assert_ok!(parser.load_uint(150));
        let claimed_nanorewards = assert_ok!(parser.load_uint(150));
        let claimed_fee_nanorewards = assert_ok!(parser.load_uint(150));
        let accrued_nanorewards = assert_ok!(parser.load_uint(150));
        let last_update_time = assert_ok!(parser.load_u64(64));

        let result = FarmDataAccrued {
            deposited_nanorewards,
            accrued_per_unit_nanorewards,
            accrued_fee_nanorewards,
            claimed_nanorewards,
            claimed_fee_nanorewards,
            accrued_nanorewards,
            last_update_time,
        };
        Ok(result)
    }

    fn key_bit_len(&self) -> usize {
        8
    }
}

static FARM_DATA_ACCRUED_LOADER: FarmDataAccruedLoader = FarmDataAccruedLoader {};
static FARM_DATA_PARAMETERS_LOADER: FarmDataParametersLoader = FarmDataParametersLoader {};

#[tokio::test]
async fn test_get_farming_minter_data() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQCVKUN-R4MnNWzZuT4U0qu7E_MJOEoCMrsXBzqgz3bWLHPB".parse()
    ));

    let stack = assert_ok!(
        contract
            .run_get_method("get_farming_minter_data", &Vec::new())
            .await
    );

    for element in stack.stack.clone() {
        log::info!("{:?}", element);
    }

    let farm_data_accrued = assert_ok!(stack.stack[10].get_dict(&FARM_DATA_ACCRUED_LOADER));
    log::info!("farm_data_accrued: {:?}", farm_data_accrued);

    let farm_data_parameters = assert_ok!(stack.stack[11].get_dict(&FARM_DATA_PARAMETERS_LOADER));
    log::info!("farm_data_parameters: {:?}", farm_data_parameters);
}

#[tokio::test]
async fn test_get_farming_data() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQBRgtldb7CftsvItA3KZk7tJ1LZFJKQHcY0wmr-WnL82_IC".parse()
    ));

    //"EQAhJy7BMg_sUHInsqypN8Na1SJXd_2IaK8k_q-84OJ8fPrg"
    //"EQBir4OJYVSWCMrZ3X6VBEtC0eh-fXje4vIPd-C7Bl_UTkmJ"
    //"EQBRgtldb7CftsvItA3KZk7tJ1LZFJKQHcY0wmr-WnL82_IC"

    let stack = assert_ok!(
        contract
            .run_get_method("get_farming_data", &Vec::new())
            .await
    );

    for element in stack.stack.clone() {
        log::info!("{:?}", element);
    }

    let loader = GenericDictLoader::new(key_extractor_u8, value_extractor_uint, 8);

    let claimed_per_unit_dict = stack.stack[4].get_dict(&loader);

    log::info!("{:?}", claimed_per_unit_dict);
}
