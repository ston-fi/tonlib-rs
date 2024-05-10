use async_trait::async_trait;
use num_bigint::BigUint;
use strum::IntoStaticStr;

use crate::address::TonAddress;
use crate::cell::{ArcCell, BagOfCells, CellBuilder, CellSlice, TonCellError};
use crate::contract::{MapCellError, MapStackError, TonContractError, TonContractInterface};
use crate::meta::MetaDataContent;
use crate::types::TvmStackEntry;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct JettonData {
    pub total_supply: BigUint,
    pub mintable: bool,
    pub admin_address: TonAddress,
    pub content: MetaDataContent,
    pub wallet_code: ArcCell,
}

#[derive(IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
enum JettonMasterMethods {
    GetJettonData,
    GetWalletAddress,
}

#[async_trait]
pub trait JettonMasterContract: TonContractInterface {
    async fn get_jetton_data(&self) -> Result<JettonData, TonContractError> {
        const JETTON_DATA_STACK_ELEMENTS: usize = 5;
        let method = JettonMasterMethods::GetJettonData.into();
        let address = self.address().clone();

        let res = self.run_get_method(method, &Vec::new()).await?;

        let stack = res.stack;
        if stack.len() == JETTON_DATA_STACK_ELEMENTS {
            let total_supply = stack[0].get_biguint().map_stack_error(method, &address)?;
            let mintable = stack[1].get_bool().map_stack_error(method, &address)?;
            let admin_address = stack[2].get_address().map_stack_error(method, &address)?;
            let cell = stack[3].get_cell().map_stack_error(method, &address)?;
            let content = read_jetton_metadata_content(cell).map_cell_error(method, &address)?;
            let wallet_code = stack[4].get_cell().map_stack_error(method, &address)?;
            Ok(JettonData {
                total_supply,
                mintable,
                admin_address,
                content,
                wallet_code,
            })
        } else {
            Err(TonContractError::InvalidMethodResultStackSize {
                method: method.to_string(),
                address: self.address().clone(),
                actual: stack.len(),
                expected: JETTON_DATA_STACK_ELEMENTS,
            })
        }
    }

    async fn get_wallet_address(
        &self,
        owner_address: &TonAddress,
    ) -> Result<TonAddress, TonContractError> {
        let method: &'static str = JettonMasterMethods::GetWalletAddress.into();
        let address = self.address().clone();
        let cell = CellBuilder::new()
            .store_address(owner_address)
            .map_cell_error(method, owner_address)?
            .build()
            .map_cell_error(method, owner_address)?;
        let cell_slice = CellSlice::full_cell(cell).map_cell_error(method, owner_address)?;
        let slice = TvmStackEntry::Slice(cell_slice);
        let res = self.run_get_method(method, &vec![slice]).await?;
        let stack = res.stack;
        if stack.len() == 1 {
            stack[0].get_address().map_stack_error(method, &address)
        } else {
            Err(TonContractError::InvalidMethodResultStackSize {
                method: method.to_string(),
                address: self.address().clone(),
                actual: stack.len(),
                expected: 1,
            })
        }
    }
}

impl<T> JettonMasterContract for T where T: TonContractInterface {}

fn read_jetton_metadata_content(cell: ArcCell) -> Result<MetaDataContent, TonCellError> {
    let mut parser = cell.parser();
    let content_representation = parser.load_byte()?;
    match content_representation {
        0 => {
            let dict = cell.reference(0)?.load_snake_formatted_dict()?;
            let converted_dict = dict
                .into_iter()
                .map(|(key, value)| (key, String::from_utf8_lossy(&value).to_string()))
                .collect();
            Ok(MetaDataContent::Internal {
                dict: converted_dict,
            }) //todo #79
        }
        1 => {
            let remaining_bytes = parser.remaining_bytes();
            let uri = parser.load_utf8(remaining_bytes)?;
            Ok(MetaDataContent::External { uri })
        }
        _ => Ok(MetaDataContent::Unsupported {
            boc: BagOfCells::from_root(cell.as_ref().clone()),
        }),
    }
}
