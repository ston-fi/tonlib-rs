pub use error::*;
pub use ipfs_loader::*;
pub use loader::*;

mod error;
mod ipfs_loader;
mod loader;

use std::collections::HashMap;
use std::fmt::Debug;

use async_trait::async_trait;
use lazy_static::lazy_static;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

use crate::cell::{ArcCell, BagOfCells, TonCellError};

struct MetaDataField {
    pub(crate) key: [u8; 32],
}

impl MetaDataField {
    fn new(name: &str) -> MetaDataField {
        let key = Self::key_from_str(name).unwrap_or([0; 32]);
        MetaDataField { key }
    }

    fn key_from_str(k: &str) -> Result<[u8; 32], MetaLoaderError> {
        let mut hasher: Sha256 = Sha256::new();
        hasher.update(k);
        let slice = &hasher.finalize()[..];
        TryInto::<[u8; 32]>::try_into(slice)
            .map_err(|e| MetaLoaderError::InternalError(e.to_string()))
    }
}

lazy_static! {
    static ref META_NAME: MetaDataField = MetaDataField::new("name");
    static ref META_DESCRIPTION: MetaDataField = MetaDataField::new("description");
    static ref META_IMAGE: MetaDataField = MetaDataField::new("image");
    static ref META_SYMBOL: MetaDataField = MetaDataField::new("symbol");
    static ref META_IMAGE_DATA: MetaDataField = MetaDataField::new("image_data");
    static ref META_DECIMALS: MetaDataField = MetaDataField::new("decimals");
    static ref META_URI: MetaDataField = MetaDataField::new("uri");
    static ref META_CONTENT_URL: MetaDataField = MetaDataField::new("content_url");
    static ref META_ATTRIBUTES: MetaDataField = MetaDataField::new("attributes");
    static ref META_SOCIAL_LINKS: MetaDataField = MetaDataField::new("social_links");
    static ref META_MARKETPLACE: MetaDataField = MetaDataField::new("marketplace");
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum MetaDataContent {
    External { uri: String },
    Internal { dict: HashMap<[u8; 32], String> },
    // TODO: Replace with cell
    Unsupported { boc: BagOfCells },
}

impl MetaDataContent {
    pub fn parse(cell: &ArcCell) -> Result<MetaDataContent, TonCellError> {
        // TODO: Refactor NFT metadata to use this method and merge collection data & item data afterwards
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
                })
            }
            1 => {
                let remaining_bytes = parser.remaining_bytes();
                let uri = parser.load_utf8(remaining_bytes)?;
                Ok(MetaDataContent::External { uri })
            }
            _ => Ok(MetaDataContent::Unsupported {
                boc: BagOfCells {
                    roots: vec![cell.clone()],
                },
            }),
        }
    }
}

pub struct MetaLoader<MetaData>
where
    MetaData: DeserializeOwned,
{
    http_client: reqwest::Client,
    ipfs_loader: IpfsLoader,
    meta_data_marker: std::marker::PhantomData<MetaData>,
}
pub type JettonMetaLoader = MetaLoader<JettonMetaData>;
pub type NftItemMetaLoader = MetaLoader<NftItemMetaData>;
pub type NftColletionMetaLoader = MetaLoader<NftCollectionMetaData>;

impl<MetaData> MetaLoader<MetaData>
where
    MetaData: DeserializeOwned,
{
    pub fn new(
        ipfs_loader_config: &IpfsLoaderConfig,
    ) -> Result<MetaLoader<MetaData>, MetaLoaderError> {
        let http_client = reqwest::Client::builder().build()?;
        let ipfs_loader = IpfsLoader::new(ipfs_loader_config)?; // Replace with actual initialization
        Ok(MetaLoader {
            http_client,
            ipfs_loader,
            meta_data_marker: std::marker::PhantomData,
        })
    }

    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Result<MetaLoader<MetaData>, MetaLoaderError> {
        let http_client = reqwest::Client::builder().build()?;
        let ipfs_loader = IpfsLoader::new(&IpfsLoaderConfig::default())?; // Replace with actual initialization
        Ok(MetaLoader {
            http_client,
            ipfs_loader,
            meta_data_marker: std::marker::PhantomData,
        })
    }

    pub async fn load_meta_from_uri(&self, uri: &str) -> Result<MetaData, MetaLoaderError> {
        log::trace!("Downloading metadata from {}", uri);
        let meta_str: String = if uri.starts_with("ipfs://") {
            let path: String = uri.chars().skip(7).collect();
            self.ipfs_loader.load_utf8_lossy(path.as_str()).await?
        } else {
            let resp = self.http_client.get(uri).send().await?;
            if resp.status().is_success() {
                resp.text().await?
            } else {
                return Err(MetaLoaderError::LoadMetaDataFailed {
                    uri: uri.to_string(),
                    status: resp.status(),
                });
            }
        };

        // Deserialize using the original meta_str
        let meta: MetaData = serde_json::from_str(&meta_str)?;

        Ok(meta)
    }
}

#[async_trait]
pub trait LoadMeta<T>
where
    T: DeserializeOwned,
{
    async fn load(&self, content: &MetaDataContent) -> Result<T, MetaLoaderError>;
}
