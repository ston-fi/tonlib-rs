use async_trait::async_trait;
use factory::TonContractFactory;
use num_bigint::{BigInt, BigUint};
use strum::IntoStaticStr;

use crate::address::TonAddress;
use crate::cell::{ArcCell, BagOfCells};
use crate::contract::{
    factory, MapCellError, MapStackError, TonContractError, TonContractInterface,
};
use crate::meta::MetaDataContent;
use crate::types::TvmStackEntry;

/// Data returned by get_static_data according to TEP-62
#[derive(Debug, Clone)]
pub struct NftItemData {
    /// if not zero, then this NFT is fully initialized and ready for interaction.
    pub init: bool,
    /// numerical index of this NFT in the collection.
    /// For collection-less NFT - arbitrary but constant value.
    pub index: BigUint,
    /// Address of the smart contract of the collection to which this NFT belongs.
    /// For collection-less NFT this parameter should be addr_none;
    pub collection_address: TonAddress,
    /// Address of the current owner of this NFT.
    pub owner_address: TonAddress,
    /// If NFT has collection - individual NFT content in any format;
    /// If NFT has no collection - NFT content in format that complies with standard TEP-64.
    pub individual_content: MetaDataContent,
}

#[derive(IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
enum NftItemContractMethods {
    GetNftData,
    GetNftContent,
}

#[async_trait]
pub trait NftItemContract: TonContractInterface {
    async fn get_nft_data(&self) -> Result<NftItemData, TonContractError> {
        let method = NftItemContractMethods::GetNftData.into();
        const NFT_DATA_STACK_ELEMENTS: usize = 5;
        let address = self.address().clone();

        let stack = self.run_get_method(method, &Vec::new()).await?.stack;
        if stack.len() == NFT_DATA_STACK_ELEMENTS {
            let init = stack[0].get_bool().map_stack_error(method, &address)?;
            let index = stack[1].get_biguint().map_stack_error(method, &address)?;
            let collection_address = stack[2].get_address().map_stack_error(method, &address)?;
            let owner_address = stack[3].get_address().map_stack_error(method, &address)?;
            let cell = stack[4].get_cell().map_stack_error(method, &address)?;

            let individual_content = read_item_metadata_content(
                self.factory(),
                &index.clone(),
                &collection_address.clone(),
                &address,
                cell,
            )
            .await?;

            Ok(NftItemData {
                init,
                index,
                collection_address,
                owner_address,
                individual_content,
            })
        } else {
            Err(TonContractError::InvalidMethodResultStackSize {
                method: method.to_string(),
                address: self.address().clone(),
                actual: stack.len(),
                expected: NFT_DATA_STACK_ELEMENTS,
            })
        }
    }

    /// Gets the serial number of the NFT item of this collection and
    /// the individual content of this NFT item and
    /// returns the full content of the NFT item in format
    /// that complies with standard TEP-64.
    async fn get_nft_content(
        &self,
        index: &BigUint,
        individual_content: BagOfCells,
    ) -> Result<BagOfCells, TonContractError> {
        let method: &'static str = NftItemContractMethods::GetNftContent.into();

        let index = BigInt::from(index.clone());

        let cell = individual_content
            .single_root()
            .map_cell_error(method, self.address())?;
        let input_stack = vec![
            TvmStackEntry::Int257(index),
            TvmStackEntry::Cell(cell.clone()),
        ];
        let stack = self.run_get_method(method, &input_stack).await?.stack;

        if stack.len() == 1 {
            let cell = stack[0]
                .get_cell()
                .map_stack_error(method, self.address())?;
            let boc = BagOfCells::from_root(cell.as_ref().clone());
            log::trace!("Got Boc: {:?}", boc);
            Ok(boc)
        } else {
            Err(TonContractError::InvalidMethodResultStackSize {
                method: method.to_string(),
                address: self.address().clone(),
                actual: stack.len(),
                expected: 1,
            })?
        }
    }
}

impl<T> NftItemContract for T where T: TonContractInterface {}

async fn read_item_metadata_content(
    factory: &TonContractFactory,
    index: &BigUint,
    collection_address: &TonAddress,
    item_address: &TonAddress,
    cell: ArcCell,
) -> Result<MetaDataContent, TonContractError> {
    let mut parser = cell.parser();
    let content_representation = parser
        .load_byte()
        .map_cell_error("get_nft_data", item_address)?;
    match content_representation {
        // On-chain content layout
        // The first byte is 0x00 and the rest is key/value dictionary.
        // Key is sha256 hash of string. Value is data encoded as described in "Data serialization" paragraph.
        0 => {
            let reference = cell
                .reference(0)
                .map_cell_error("get_nft_data", item_address)?;
            let dict = reference
                .load_snake_formatted_dict()
                .map_cell_error("get_nft_data", item_address)?;
            let converted_dict = dict
                .into_iter()
                .map(|(key, value)| (key, String::from_utf8_lossy(&value).to_string()))
                .collect();
            Ok(MetaDataContent::Internal {
                dict: converted_dict,
            }) //todo #79s
        }
        // Off-chain content layout
        // The first byte is 0x01 and the rest is the URI pointing to the JSON document containing the token metadata.
        // The URI is encoded as ASCII. If the URI does not fit into one cell, then it uses the "Snake format"
        //  described in the "Data serialization" paragraph, the snake-format-prefix 0x00 is dropped.
        1 => {
            let remaining_bytes = parser.remaining_bytes();
            let uri = parser
                .load_utf8(remaining_bytes)
                .map_cell_error("get_nft_data", item_address)?;
            Ok(MetaDataContent::External { uri })
        }

        // Semi-chain content layout
        // Data encoded as described in "2. On-chain content layout".
        // The dictionary must have uri key with a value containing the URI pointing to the JSON document with token metadata.
        // Clients in this case should merge the keys of the on-chain dictionary and off-chain JSON doc.
        _ => {
            let contract = factory.get_contract(collection_address);
            let boc = BagOfCells::from_root(cell.as_ref().clone());
            let nft_content = contract.get_nft_content(index, boc.clone()).await?;
            let cell = nft_content
                .single_root()
                .map_cell_error("get_nft_content", item_address)?
                .clone();
            let uri = cell
                .load_snake_formatted_string()
                .map_cell_error("get_nft_content", item_address)?;
            Ok(MetaDataContent::External {
                uri: uri.to_string(),
            })
        }
    }
}
