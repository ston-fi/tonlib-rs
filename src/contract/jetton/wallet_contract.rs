use async_trait::async_trait;

use num_bigint::BigUint;

use crate::contract::TonContractInterface;
use crate::{
    address::TonAddress,
    cell::BagOfCells,
    contract::{MapStackError, TonContractError},
};

#[derive(Debug, Clone)]
pub struct WalletData {
    pub balance: BigUint,
    pub owner_address: TonAddress,
    pub master_address: TonAddress,
    pub wallet_code: BagOfCells,
}

#[async_trait]
pub trait JettonWalletContract: TonContractInterface {
    async fn get_wallet_data(&self) -> Result<WalletData, TonContractError> {
        const WALLET_DATA_STACK_ELEMENTS: usize = 4;
        let method_name = "get_wallet_data";
        let address = self.address().clone();

        let res = self.run_get_method(method_name, &Vec::new()).await?;

        let stack = res.stack;
        if stack.elements.len() == WALLET_DATA_STACK_ELEMENTS {
            let balance = stack
                .get_biguint(0)
                .map_stack_error(method_name, &address)?;
            let owner_address = stack
                .get_address(1)
                .map_stack_error(method_name, &address)?;
            let master_address = stack
                .get_address(2)
                .map_stack_error(method_name, &address)?;
            let wallet_code = stack.get_boc(3).map_stack_error(method_name, &address)?;

            Ok(WalletData {
                balance,
                owner_address,
                master_address,
                wallet_code,
            })
        } else {
            Err(TonContractError::InvalidMethodResultStackSize {
                method: "get_wallet_data".to_string(),
                address: self.address().clone(),

                actual: stack.elements.len(),
                expected: WALLET_DATA_STACK_ELEMENTS,
            })
        }
    }
}

impl<T> JettonWalletContract for T where T: TonContractInterface {}
