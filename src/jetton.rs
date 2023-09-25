use async_trait::async_trait;
use std::collections::HashMap;

use num_bigint::BigUint;

use crate::{
    cell::TonCellError,
    contract::{MapCellError, MapStackError, TonContract, TonContractError},
    tl::TvmStackEntry,
};

use crate::cell::{BagOfCells, CellBuilder};
use crate::tl::TvmSlice;
use crate::tl::TvmStackEntry::Slice;
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
    async fn get_jetton_data(&self) -> Result<JettonData, TonContractError>;
    async fn get_wallet_address(
        &self,
        owner_address: &TonAddress,
    ) -> Result<TonAddress, TonContractError>;
}

#[async_trait]
impl JettonMasterContract for TonContract {
    async fn get_jetton_data(&self) -> Result<JettonData, TonContractError> {
        const JETTON_DATA_STACK_ELEMENTS: usize = 5;
        let method_name = "get_jetton_data";
        let address = self.address().clone();

        let res = self.run_get_method(method_name, &Vec::new()).await?;

        let stack = res.stack;
        if stack.elements.len() == JETTON_DATA_STACK_ELEMENTS {
            let total_supply = stack
                .get_biguint(0)
                .map_stack_error(method_name, &address)?;
            let mintable = stack.get_i32(1).map_stack_error(method_name, &address)? != 0;
            let admin_address = stack
                .get_address(2)
                .map_stack_error(method_name, &address)?;
            let boc = stack.get_boc(3).map_stack_error(method_name, &address)?;
            let content =
                read_jetton_metadata_content(&boc).map_cell_error(method_name, &address)?;
            let wallet_code = stack.get_boc(4).map_stack_error(method_name, &address)?;

            Ok(JettonData {
                total_supply,
                mintable,
                admin_address,
                content,
                wallet_code,
            })
        } else {
            Err(TonContractError::InvalidMethodResultStackSize {
                method: method_name.to_string(),
                address: self.address().clone(),

                actual: stack.elements.len(),
                expected: JETTON_DATA_STACK_ELEMENTS,
            })
        }
    }

    async fn get_wallet_address(
        &self,
        owner_address: &TonAddress,
    ) -> Result<TonAddress, TonContractError> {
        let method_name = "get_wallet_address";
        let address = self.address().clone();

        let slice = match build_get_wallet_address_payload(owner_address) {
            Ok(slice) => Ok(slice),
            Err(e) => Err(TonContractError::CellError {
                method: method_name.to_string(),
                address: self.address().clone(),
                error: e,
            }),
        }?;

        let res = self.run_get_method(method_name, &vec![slice]).await?;

        let stack = res.stack;
        if stack.elements.len() == 1 {
            stack.get_address(0).map_stack_error(method_name, &address)
        } else {
            Err(TonContractError::InvalidMethodResultStackSize {
                method: method_name.to_string(),
                address: self.address().clone(),

                actual: stack.elements.len(),
                expected: 1,
            })
        }
    }
}

fn build_get_wallet_address_payload(
    owner_address: &TonAddress,
) -> Result<TvmStackEntry, TonCellError> {
    let cell = CellBuilder::new().store_address(owner_address)?.build()?;
    let boc = BagOfCells::from_root(cell);
    let slice = Slice {
        slice: TvmSlice {
            bytes: boc.serialize(true)?,
        },
    };
    Ok(slice)
}

fn read_jetton_metadata_content(boc: &BagOfCells) -> Result<MetaDataContent, TonCellError> {
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
    async fn get_wallet_data(&self) -> Result<WalletData, TonContractError>;
}

#[async_trait]
impl JettonWalletContract for TonContract {
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
