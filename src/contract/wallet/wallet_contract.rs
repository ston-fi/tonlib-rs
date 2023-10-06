use async_trait::async_trait;

use crate::contract::{MapStackError, TonContractError, TonContractInterface};

#[async_trait]
pub trait TonWalletContract: TonContractInterface {
    async fn seqno(&self) -> Result<u32, TonContractError> {
        let res = self.run_get_method("seqno", &Vec::new()).await?;
        let stack = res.stack;
        if stack.elements.len() != 1 {
            Err(TonContractError::InvalidMethodResultStackSize {
                method: "seqno".to_string(),
                address: self.address().clone(),
                actual: stack.elements.len(),
                expected: 1,
            })
        } else {
            let result = stack.get_i32(0).map_stack_error("seqno", self.address())? as u32;
            Ok(result)
        }
    }

    async fn get_public_key(&self) -> Result<Vec<u8>, TonContractError> {
        let res = self.run_get_method("get_public_key", &Vec::new()).await?;
        let stack = res.stack;
        if stack.elements.len() != 1 {
            Err(TonContractError::InvalidMethodResultStackSize {
                method: "get_public_key".to_string(),
                address: self.address().clone(),
                actual: stack.elements.len(),
                expected: 1,
            })
        } else {
            let pub_key = stack
                .get_biguint(0)
                .map_stack_error("get_public_key", self.address())?;
            Ok(pub_key.to_bytes_be())
        }
    }
}

impl<T> TonWalletContract for T where T: TonContractInterface {}
