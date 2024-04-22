use async_trait::async_trait;
use num_bigint::BigUint;
use num_traits::Zero;
use strum::IntoStaticStr;

use crate::address::TonAddress;
use crate::cell::{ArcCell, BagOfCells};
use crate::contract::factory::TonContractFactory;
use crate::contract::{
    MapCellError, MapStackError, NftItemContract, TonContractError, TonContractInterface,
};
use crate::meta::MetaDataContent;
use crate::types::TvmStackEntry;

/// Data returned by get_collection_data according to TEP-62
#[derive(Debug, Clone)]
pub struct NftCollectionData {
    /// The count of currently deployed NFT items in collection.
    /// Generally, collection should issue NFT with sequential indexes (see Rationale(2) ).
    ///  -1 value of next_item_index is used to indicate non-sequential collections,
    /// such collections should provide their own way for index generation / item enumeration.
    pub next_item_index: i64,
    /// collection_content - collection content in a format that complies with standard TEP-64.
    pub collection_content: MetaDataContent,
    /// owner_address - collection owner address, zero address if no owner.
    pub owner_address: TonAddress,
}

#[derive(IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
enum NftCollectionMethods {
    GetCollectionData,
    GetNftAddressByIndex,
}

#[async_trait]
pub trait NftCollectionContract: TonContractInterface {
    /// Returns nft collection data.
    async fn get_collection_data(&self) -> Result<NftCollectionData, TonContractError> {
        const NFT_COLLECTION_STACK_ELEMENTS: usize = 3;
        let method = NftCollectionMethods::GetCollectionData.into();
        let address = self.address().clone();

        let stack = self.run_get_method(method, &Vec::new()).await?.stack;
        if stack.len() == NFT_COLLECTION_STACK_ELEMENTS {
            let next_item_index = stack[0].get_i64().map_stack_error(method, &address)?;
            let cell = stack[1].get_cell().map_stack_error(method, &address)?;
            let collection_content =
                read_collection_metadata_content(self.factory(), &address, cell).await?;
            let owner_address = stack[2].get_address().map_stack_error(method, &address)?;

            Ok(NftCollectionData {
                next_item_index,
                collection_content,
                owner_address,
            })
        } else {
            Err(TonContractError::InvalidMethodResultStackSize {
                method: method.to_string(),
                address: self.address().clone(),
                actual: stack.len(),
                expected: NFT_COLLECTION_STACK_ELEMENTS,
            })
        }
    }

    /// Gets the serial number of the NFT item of this collection and
    /// returns the address (TonAddress) of this NFT item smart contract.
    async fn get_nft_address_by_index(&self, index: i64) -> Result<TonAddress, TonContractError> {
        let method = NftCollectionMethods::GetNftAddressByIndex.into();
        let input_stack = vec![TvmStackEntry::Int64(index)];
        let stack = self.run_get_method(method, &input_stack).await?.stack;

        if stack.len() == 1 {
            stack[0]
                .get_address()
                .map_stack_error(method, self.address())
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

impl<T> NftCollectionContract for T where T: TonContractInterface {}
async fn read_collection_metadata_content(
    factory: &TonContractFactory,
    collection_address: &TonAddress,
    cell: ArcCell,
) -> Result<MetaDataContent, TonContractError> {
    let mut parser = cell.parser();
    let content_representation = parser
        .load_byte()
        .map_cell_error("get_collection_data", collection_address)?;
    match content_representation {
        // Off-chain content layout
        // The first byte is 0x01 and the rest is the URI pointing to the JSON document containing the token metadata.
        // The URI is encoded as ASCII. If the URI does not fit into one cell, then it uses the "Snake format"
        //  described in the "Data serialization" paragraph, the snake-format-prefix 0x00 is dropped.
        0 => {
            let reference = cell
                .reference(0)
                .map_cell_error("get_collection_data", collection_address)?;
            let dict = reference
                .load_snake_formatted_dict()
                .map_cell_error("get_collection_data", collection_address)?;
            let converted_dict = dict
                .into_iter()
                .map(|(key, value)| (key, String::from_utf8_lossy(&value).to_string()))
                .collect();
            Ok(MetaDataContent::Internal {
                dict: converted_dict,
            }) //todo #79
        }
        // On-chain content layout
        // The first byte is 0x00 and the rest is key/value dictionary.
        // Key is sha256 hash of string. Value is data encoded as described in "Data serialization" paragraph.
        1 => {
            let remaining_bytes = parser.remaining_bytes();
            let uri = parser
                .load_utf8(remaining_bytes)
                .map_cell_error("get_collection_data", collection_address)?;
            Ok(MetaDataContent::External { uri })
        }

        // Semi-chain content layout
        // Data encoded as described in "2. On-chain content layout".
        // The dictionary must have uri key with a value containing the URI pointing to the JSON document with token metadata.
        // Clients in this case should merge the keys of the on-chain dictionary and off-chain JSON doc.
        _ => {
            let contract = factory.get_contract(collection_address);
            let boc = BagOfCells::from_root(cell.as_ref().clone());
            let nft_content = contract
                .get_nft_content(&BigUint::zero(), boc.clone())
                .await?;
            let cell = nft_content
                .single_root()
                .map_cell_error("get_nft_content", collection_address)?
                .clone();
            let uri = cell
                .load_snake_formatted_string()
                .map_cell_error("get_nft_content", collection_address)?;
            Ok(MetaDataContent::External {
                uri: uri.to_string(),
            })
        }
    }
}
