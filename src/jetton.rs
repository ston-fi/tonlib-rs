use anyhow::anyhow;
use async_trait::async_trait;
use std::collections::HashMap;

use num_bigint::BigUint;

use crate::contract::TonContract;

use crate::cell::{BagOfCells, CellBuilder};
use crate::tl::stack::TvmSlice;
use crate::tl::stack::TvmStackEntry::Slice;
use crate::{address::TonAddress, meta::MetaDataContent};

// Constants from jetton reference implementation:
// https://github.com/ton-blockchain/token-contract/blob/main/ft/op-codes.fc
pub const JETTON_TRANSFER: u32 = 0xf8a7ea5;
pub const JETTON_TRANSFER_NOTIFICATION: u32 = 0x7362d09c;
pub const JETTON_INTERNAL_TRANSFER: u32 = 0x178d4519;
pub const JETTON_EXCESSES: u32 = 0xd53276db;
pub const JETTON_BURN: u32 = 0x595f07bc;
pub const JETTON_BURN_NOTIFICATION: u32 = 0x7bdd97de;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct JettonData {
    pub total_supply: BigUint,
    pub mintable: bool,
    pub admin_address: TonAddress,
    pub content: MetaDataContent,
    pub wallet_code: BagOfCells,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum JettonContent {
    External { uri: String },
    Internal { dict: HashMap<String, String> },
    Unsupported { boc: BagOfCells },
}

#[async_trait]
pub trait JettonMasterContract {
    async fn get_jetton_data(&self) -> anyhow::Result<JettonData>;
    async fn get_wallet_address(&self, owner_address: &TonAddress) -> anyhow::Result<TonAddress>;
}

#[async_trait]
impl JettonMasterContract for TonContract {
    async fn get_jetton_data(&self) -> anyhow::Result<JettonData> {
        let res = self.run_get_method("get_jetton_data", &Vec::new()).await?;
        let stack = res.stack;
        if stack.elements.len() != 5 {
            Err(anyhow!(
                "Invalid get_jetton_data result from {}, expected 5 elements, got {}",
                self.address(),
                stack.elements.len()
            ))
        } else {
            let result = JettonData {
                total_supply: stack.get_biguint(0)?,
                mintable: stack.get_i32(1)? != 0,
                admin_address: stack
                    .get_boc(2)?
                    .single_root()?
                    .parse_fully(|r: &mut crate::cell::CellParser<'_>| r.load_address())?,
                content: read_jetton_metadata_content(&stack.get_boc(3)?)?,
                wallet_code: stack.get_boc(4)?,
            };
            Ok(result)
        }
    }

    async fn get_wallet_address(&self, owner_address: &TonAddress) -> anyhow::Result<TonAddress> {
        let cell = CellBuilder::new().store_address(owner_address)?.build()?;
        let boc = BagOfCells::from_root(cell);
        let slice = Slice {
            slice: TvmSlice {
                bytes: boc.serialize(true)?,
            },
        };

        let res = self
            .run_get_method("get_wallet_address", &vec![slice])
            .await?;
        let stack = res.stack;
        if stack.elements.len() != 1 {
            Err(anyhow!(
                "Invalid get_wallet_address result from {}, expected 1 element, got {}",
                self.address(),
                stack.elements.len()
            ))
        } else {
            let result = stack
                .get_boc(0)?
                .single_root()?
                .parse_fully(|r| r.load_address())?;
            Ok(result)
        }
    }
}

fn read_jetton_metadata_content(boc: &BagOfCells) -> anyhow::Result<MetaDataContent> {
    if let Ok(root) = boc.single_root() {
        let mut reader = root.parser();
        let content_representation = reader.load_byte()?;
        match content_representation {
            0 => {
                let dict = root.reference(0)?.load_snake_formatted_dict()?;
                Ok(MetaDataContent::Internal { dict })
            }
            1 => {
                let uri = reader.load_string(reader.remaining_bytes())?;
                Ok(MetaDataContent::External { uri })
            }
            _ => Ok(MetaDataContent::Unsupported { boc: boc.clone() }),
        }
    } else {
        Ok(MetaDataContent::Unsupported { boc: boc.clone() })
    }
}

#[derive(Debug, Clone)]
pub struct WalletData {
    pub balance: BigUint,
    pub owner_address: TonAddress,
    pub master_address: TonAddress,
    pub wallet_code: BagOfCells,
}

#[async_trait]
pub trait JettonWalletContract {
    async fn get_wallet_data(&self) -> anyhow::Result<WalletData>;
}

#[async_trait]
impl JettonWalletContract for TonContract {
    async fn get_wallet_data(&self) -> anyhow::Result<WalletData> {
        let res = self.run_get_method("get_wallet_data", &Vec::new()).await?;
        let stack = res.stack;
        if stack.elements.len() != 4 {
            Err(anyhow!(
                "Invalid get_wallet_data result from {}, expected 4 elements, got {}",
                self.address(),
                stack.elements.len()
            ))
        } else {
            let result = WalletData {
                balance: stack.get_biguint(0)?,
                owner_address: stack
                    .get_boc(1)?
                    .single_root()?
                    .parse_fully(|r| r.load_address())?,
                master_address: stack
                    .get_boc(2)?
                    .single_root()?
                    .parse_fully(|r| r.load_address())?,
                wallet_code: stack.get_boc(3)?,
            };
            Ok(result)
        }
    }
}
