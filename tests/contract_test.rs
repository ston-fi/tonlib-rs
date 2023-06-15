use anyhow::anyhow;
use async_trait::async_trait;
use num_bigint::BigUint;

use tonlib::address::TonAddress;
use tonlib::contract::TonContract;

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
pub trait PoolContract {
    async fn get_pool_data(&self) -> anyhow::Result<PoolData>;

    async fn invalid_method(&self) -> anyhow::Result<()>;
}

#[async_trait]
impl PoolContract for TonContract {
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

#[tokio::test]
async fn client_get_pool_data_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let contract = TonContract::new(
        &client,
        &"EQD9b5pxv6nptJmD1-c771oRV98h_mky-URkDn5BJpY2sTJ-".parse()?,
    );
    let pool_data = contract.get_pool_data().await?;
    println!("pool data: {:?}", pool_data);
    let invalid_result = contract.invalid_method().await;
    assert!(invalid_result.is_err());
    Ok(())
}
