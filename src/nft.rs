use std::fmt::Debug;

use anyhow::anyhow;
use async_trait::async_trait;
use num_bigint::BigUint;
use num_traits::Zero;

use crate::{
    address::TonAddress,
    cell::BagOfCells,
    client::TonClient,
    contract::TonContract,
    meta::MetaDataContent,
    tl::stack::{TvmCell, TvmNumber, TvmStackEntry},
};

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

#[async_trait]
pub trait NftItemContract {
    /// Gets nft item data.
    async fn get_nft_data(&self) -> anyhow::Result<NftItemData>;
    async fn get_nft_content(
        &self,
        index: &BigUint,
        individual_content: BagOfCells,
    ) -> anyhow::Result<BagOfCells>;
}
#[async_trait]
pub trait NftCollectionContract {
    /// Returns nft collection data.
    async fn get_collection_data(&self) -> anyhow::Result<NftCollectionData>;
    ///Gets the serial number of the NFT item of this collection and
    ///returns the address (TonAddress) of this NFT item smart contract.
    async fn get_nft_address_by_index(&self, index: i64) -> anyhow::Result<TonAddress>;
}

#[async_trait]
impl NftItemContract for TonContract {
    async fn get_nft_data(&self) -> anyhow::Result<NftItemData> {
        const NFT_DATA_STACK_ELEMENTS: usize = 5;

        let stack = self
            .run_get_method("get_nft_data", &Vec::new())
            .await?
            .stack;
        if stack.elements.len() == NFT_DATA_STACK_ELEMENTS {
            let index = stack.get_biguint(1)?;
            let collection_address = stack
                .get_boc(2)?
                .single_root()?
                .parse_fully(|r| r.load_address())?;
            let result: NftItemData = NftItemData {
                init: stack.get_i32(0)? == 1,
                index: index.clone(),
                collection_address: collection_address.clone(),
                owner_address: stack
                    .get_boc(3)?
                    .single_root()?
                    .parse_fully(|r| r.load_address())?,
                individual_content: read_item_metadata_content(
                    self.client(),
                    &index.clone(),
                    &collection_address.clone(),
                    &stack.get_boc(4)?,
                )
                .await?,
            };
            Ok(result)
        } else {
            Err(anyhow!(
                "Invalid result size: {}, expected {}",
                stack.elements.len(),
                NFT_DATA_STACK_ELEMENTS
            ))
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
    ) -> anyhow::Result<BagOfCells> {
        let input_stack = vec![
            (TvmStackEntry::Number {
                number: TvmNumber {
                    number: index.clone().to_string(),
                },
            }),
            (TvmStackEntry::Cell {
                cell: TvmCell {
                    bytes: individual_content.serialize(false)?, // todo support crc32c
                },
            }),
        ];
        let stack = self
            .run_get_method("get_nft_content", &input_stack)
            .await?
            .stack;

        if stack.elements.len() == 1 {
            let boc = stack.get_boc(0)?;
            log::trace!("Got Boc: {:?}", boc);
            Ok(boc)
        } else {
            Err(anyhow!(
                "Invalid result size: {}, expected 1",
                stack.elements.len()
            ))
        }
    }
}

async fn read_item_metadata_content(
    client: &TonClient,
    index: &BigUint,
    collection_address: &TonAddress,
    boc: &BagOfCells,
) -> anyhow::Result<MetaDataContent> {
    if let Ok(root) = boc.single_root() {
        let mut reader = root.parser();
        let content_representation = reader.load_byte()?;
        match content_representation {
            // Off-chain content layout
            // The first byte is 0x01 and the rest is the URI pointing to the JSON document containing the token metadata.
            // The URI is encoded as ASCII. If the URI does not fit into one cell, then it uses the "Snake format"
            //  described in the "Data serialization" paragraph, the snake-format-prefix 0x00 is dropped.
            0 => {
                let dict = root.reference(0)?.load_snake_formatted_dict()?;
                Ok(MetaDataContent::Internal { dict })
            }
            // On-chain content layout
            // The first byte is 0x00 and the rest is key/value dictionary.
            // Key is sha256 hash of string. Value is data encoded as described in "Data serialization" paragraph.
            1 => {
                let uri = reader.load_string(reader.remaining_bytes())?;
                Ok(MetaDataContent::External { uri })
            }

            // Semi-chain content layout
            // Data encoded as described in "2. On-chain content layout".
            // The dictionary must have uri key with a value containing the URI pointing to the JSON document with token metadata.
            // Clients in this case should merge the keys of the on-chain dictionary and off-chain JSON doc.
            _ => {
                let collection_contract = TonContract::new(client, collection_address);
                let cell = collection_contract
                    .get_nft_content(index, boc.clone())
                    .await?
                    .single_root()?
                    .clone();
                let uri = cell.load_snake_formatted_string()?;
                Ok(MetaDataContent::External {
                    uri: uri.to_string(),
                })
            }
        }
    } else {
        Ok(MetaDataContent::Unsupported { boc: boc.clone() })
    }
}

#[async_trait]
impl NftCollectionContract for TonContract {
    async fn get_collection_data(&self) -> anyhow::Result<NftCollectionData> {
        const NFT_COLLECTION_STACK_ELEMENTS: usize = 3;

        let stack = self
            .run_get_method("get_collection_data", &Vec::new())
            .await?
            .stack;
        if stack.elements.len() == NFT_COLLECTION_STACK_ELEMENTS {
            let result: NftCollectionData = NftCollectionData {
                next_item_index: stack.get_i64(0)?,

                collection_content: read_collection_metadata_content(
                    self.client(),
                    self.address(),
                    &stack.get_boc(1)?,
                )
                .await?,
                owner_address: stack
                    .get_boc(2)?
                    .single_root()?
                    .parse_fully(|r| r.load_address())?,
            };
            Ok(result)
        } else {
            Err(anyhow!(
                "Invalid result size: {}, expected {}",
                stack.elements.len(),
                NFT_COLLECTION_STACK_ELEMENTS
            ))
        }
    }

    async fn get_nft_address_by_index(&self, index: i64) -> anyhow::Result<TonAddress> {
        let input_stack = vec![
            (TvmStackEntry::Number {
                number: TvmNumber {
                    number: index.to_string(),
                },
            }),
        ];
        let stack = self
            .run_get_method("get_nft_address_by_index", &input_stack)
            .await?
            .stack;

        if stack.elements.len() == 1 {
            stack
                .get_boc(0)?
                .single_root()?
                .parse_fully(|r| r.load_address())
        } else {
            Err(anyhow!(
                "Invalid result size: {}, expected 1",
                stack.elements.len()
            ))
        }
    }
}
async fn read_collection_metadata_content(
    client: &TonClient,
    collection_address: &TonAddress,
    boc: &BagOfCells,
) -> anyhow::Result<MetaDataContent> {
    if let Ok(root) = boc.single_root() {
        let mut reader = root.parser();
        let content_representation = reader.load_byte()?;
        match content_representation {
            // Off-chain content layout
            // The first byte is 0x01 and the rest is the URI pointing to the JSON document containing the token metadata.
            // The URI is encoded as ASCII. If the URI does not fit into one cell, then it uses the "Snake format"
            //  described in the "Data serialization" paragraph, the snake-format-prefix 0x00 is dropped.
            0 => {
                let dict = root.reference(0)?.load_snake_formatted_dict()?;
                Ok(MetaDataContent::Internal { dict })
            }
            // On-chain content layout
            // The first byte is 0x00 and the rest is key/value dictionary.
            // Key is sha256 hash of string. Value is data encoded as described in "Data serialization" paragraph.
            1 => {
                let uri = reader.load_string(reader.remaining_bytes())?;
                Ok(MetaDataContent::External { uri })
            }

            // Semi-chain content layout
            // Data encoded as described in "2. On-chain content layout".
            // The dictionary must have uri key with a value containing the URI pointing to the JSON document with token metadata.
            // Clients in this case should merge the keys of the on-chain dictionary and off-chain JSON doc.
            _ => {
                let collection_contract = TonContract::new(client, collection_address);
                let cell = collection_contract
                    .get_nft_content(&BigUint::zero(), boc.clone())
                    .await?
                    .single_root()?
                    .clone();
                let uri = cell.load_snake_formatted_string()?;
                Ok(MetaDataContent::External {
                    uri: uri.to_string(),
                })
            }
        }
    } else {
        Ok(MetaDataContent::Unsupported { boc: boc.clone() })
    }
}
