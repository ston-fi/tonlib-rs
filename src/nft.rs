use anyhow::anyhow;
use async_trait::async_trait;

use crate::{
    address::TonAddress,
    cell::BagOfCells,
    contract::TonContract,
    tl::stack::{TvmNumber, TvmStackEntry},
};

/// Data returned by get_static_data according to TEP-62
pub struct NftItemData {
    /// if not zero, then this NFT is fully initialized and ready for interaction.
    pub init: bool,
    /// numerical index of this NFT in the collection.
    /// For collection-less NFT - arbitrary but constant value.
    pub index: i64,
    /// Address of the smart contract of the collection to which this NFT belongs.
    /// For collection-less NFT this parameter should be addr_none;
    pub collection_address: TonAddress,
    /// Address of the current owner of this NFT.
    pub owner_address: TonAddress,
    /// If NFT has collection - individual NFT content in any format;
    /// If NFT has no collection - NFT content in format that complies with standard TEP-64.
    pub individual_content: BagOfCells,
}

/// Data returned by get_collection_dataaccording to TEP-62
pub struct NftCollectionData {
    /// The count of currently deployed NFT items in collection.
    /// Generally, collection should issue NFT with sequential indexes (see Rationale(2) ).
    ///  -1 value of next_item_index is used to indicate non-sequential collections,
    /// such collections should provide their own way for index generation / item enumeration.
    pub next_item_index: i64,
    /// collection_content - collection content in a format that complies with standard TEP-64.
    pub collection_content: BagOfCells,
    /// owner_address - collection owner address, zero address if no owner.
    pub owner_address: TonAddress,
}

#[async_trait]
pub trait NftItemContract {
    /// Gets nft item data.
    async fn get_nft_data(&self) -> anyhow::Result<NftItemData>;
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
            let result: NftItemData = NftItemData {
                init: stack.get_i32(0)? == 1,
                index: stack.get_i64(1)?,
                collection_address: stack
                    .get_boc(2)?
                    .single_root()?
                    .parse_fully(|r| r.load_address())?,
                owner_address: stack
                    .get_boc(3)?
                    .single_root()?
                    .parse_fully(|r| r.load_address())?,
                individual_content: stack.get_boc(4)?,
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
                collection_content: stack.get_boc(1)?,
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
