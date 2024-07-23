use async_trait::async_trait;
use strum::IntoStaticStr;

use crate::contract::{MapStackError, TonContractError, TonContractInterface};

#[derive(IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
enum WalletContractMethods {
    Seqno,
    GetPublicKey,
}

#[async_trait]
pub trait TonWalletContract: TonContractInterface {
    async fn seqno(&self) -> Result<u32, TonContractError> {
        let method: &str = WalletContractMethods::Seqno.into();
        let res = self.run_get_method("seqno", &Vec::new()).await?;
        let stack = res.stack;
        if stack.len() != 1 {
            Err(TonContractError::InvalidMethodResultStackSize {
                method: method.to_string(),
                address: self.address().clone(),
                actual: stack.len(),
                expected: 1,
            })
        } else {
            let result = stack[0].get_i64().map_stack_error(method, self.address())? as u32;
            Ok(result)
        }
    }

    async fn get_public_key(&self) -> Result<Vec<u8>, TonContractError> {
        let method: &str = WalletContractMethods::GetPublicKey.into();
        let res = self.run_get_method(method, &Vec::new()).await?;
        let stack = res.stack;
        if stack.len() != 1 {
            Err(TonContractError::InvalidMethodResultStackSize {
                method: method.to_string(),
                address: self.address().clone(),
                actual: stack.len(),
                expected: 1,
            })
        } else {
            let pub_key = stack[0]
                .get_biguint()
                .map_stack_error("get_public_key", self.address())?;
            Ok(pub_key.to_bytes_be())
        }
    }
}

impl<T> TonWalletContract for T where T: TonContractInterface {}
