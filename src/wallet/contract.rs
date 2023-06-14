use anyhow::anyhow;
use async_trait::async_trait;

use crate::{client::TonFunctions, contract::TonContract};

#[async_trait]
pub trait TonWalletContract {
    async fn seqno(&self) -> anyhow::Result<u32>;
    async fn get_public_key(&self) -> anyhow::Result<Vec<u8>>;
}

#[async_trait]
impl<C: TonFunctions + Send + Sync> TonWalletContract for TonContract<'_, C> {
    async fn seqno(&self) -> anyhow::Result<u32> {
        let res = self.run_get_method("seqno", &Vec::new()).await?;
        let stack = res.stack;
        if stack.elements.len() != 1 {
            Err(anyhow!(
                "Invalid seqno result from {}, expected 1 elements, got {}",
                self.address(),
                stack.elements.len()
            ))
        } else {
            let result = stack.get_i32(0)? as u32;
            Ok(result)
        }
    }

    async fn get_public_key(&self) -> anyhow::Result<Vec<u8>> {
        let res = self.run_get_method("get_public_key", &Vec::new()).await?;
        let stack = res.stack;
        if stack.elements.len() != 1 {
            Err(anyhow!(
                "Invalid get_public_key result from {}, expected 1 elements, got {}",
                self.address(),
                stack.elements.len()
            ))
        } else {
            let pub_key = stack.get_biguint(0)?;
            Ok(pub_key.to_bytes_be())
        }
    }
}
