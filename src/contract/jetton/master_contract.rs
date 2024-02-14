use async_trait::async_trait;
use num_bigint::BigUint;
use strum::IntoStaticStr;

use crate::address::TonAddress;
use crate::cell::{BagOfCells, CellBuilder, TonCellError};
use crate::contract::{MapCellError, MapStackError, TonContractError, TonContractInterface};
use crate::meta::MetaDataContent;
use crate::tl::{TvmSlice, TvmStackEntry};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct JettonData {
    pub total_supply: BigUint,
    pub mintable: bool,
    pub admin_address: TonAddress,
    pub content: MetaDataContent,
    pub wallet_code: BagOfCells,
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
        if stack.elements.len() == JETTON_DATA_STACK_ELEMENTS {
            let total_supply = stack.get_biguint(0).map_stack_error(method, &address)?;
            let mintable = stack.get_i32(1).map_stack_error(method, &address)? != 0;
            let admin_address = stack.get_address(2).map_stack_error(method, &address)?;
            let boc = stack.get_boc(3).map_stack_error(method, &address)?;
            let content = read_jetton_metadata_content(&boc).map_cell_error(method, &address)?;
            let wallet_code = stack.get_boc(4).map_stack_error(method, &address)?;

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

                actual: stack.elements.len(),
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

        let slice = match build_get_wallet_address_payload(owner_address) {
            Ok(slice) => Ok(slice),
            Err(e) => Err(TonContractError::CellError {
                method: method.to_string(),
                address: self.address().clone(),
                error: e,
            }),
        }?;

        let res = self.run_get_method(method, &vec![slice]).await?;

        let stack = res.stack;
        if stack.elements.len() == 1 {
            stack.get_address(0).map_stack_error(method, &address)
        } else {
            Err(TonContractError::InvalidMethodResultStackSize {
                method: method.to_string(),
                address: self.address().clone(),

                actual: stack.elements.len(),
                expected: 1,
            })
        }
    }
}

impl<T> JettonMasterContract for T where T: TonContractInterface {}

fn build_get_wallet_address_payload(
    owner_address: &TonAddress,
) -> Result<TvmStackEntry, TonCellError> {
    let cell = CellBuilder::new().store_address(owner_address)?.build()?;
    let boc = BagOfCells::from_root(cell);
    let slice = TvmStackEntry::Slice {
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
                let converted_dict = dict
                    .into_iter()
                    .map(|(key, value)| (key, String::from_utf8_lossy(&value).to_string()))
                    .collect();
                Ok(MetaDataContent::Internal {
                    dict: converted_dict,
                }) //todo #79
            }
            1 => {
                let uri = reader.load_utf8(reader.remaining_bytes())?;
                Ok(MetaDataContent::External { uri })
            }
            _ => Ok(MetaDataContent::Unsupported { boc: boc.clone() }),
        }
    } else {
        Ok(MetaDataContent::Unsupported { boc: boc.clone() })
    }
}
