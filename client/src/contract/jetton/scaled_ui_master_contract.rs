use async_trait::async_trait;
use num_bigint::BigInt;

use crate::contract::{MapStackError, TonContractError, TonContractInterface};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct DisplayMultiplier {
    pub numerator: BigInt,
    pub denominator: BigInt,
}

#[async_trait]
pub trait ScaledUiMasterContract: TonContractInterface {
    async fn get_display_multiplier(&self) -> Result<DisplayMultiplier, TonContractError> {
        const EXPECTED_STACK_SIZE: usize = 2;
        let method = "get_display_multiplier";
        let address = self.address().clone();

        let res = self.run_get_method(method, Vec::new()).await?;

        let stack = res.stack;
        if stack.len() == EXPECTED_STACK_SIZE {
            let numerator = stack[0].get_bigint().map_stack_error(method, &address)?;
            let denominator = stack[1].get_bigint().map_stack_error(method, &address)?;
            Ok(DisplayMultiplier {
                numerator,
                denominator,
            })
        } else {
            Err(TonContractError::InvalidMethodResultStackSize {
                method: method.to_string(),
                address: self.address().clone(),
                actual: stack.len(),
                expected: EXPECTED_STACK_SIZE,
            })
        }
    }
}

impl<T> ScaledUiMasterContract for T where T: TonContractInterface {}
