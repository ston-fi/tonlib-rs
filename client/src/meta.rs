pub use error::*;
pub use ipfs_loader::*;
pub use loader::*;
use serde_json::Value;
use tonlib_core::cell::{ArcCell, BagOfCells, MapTonCellError, TonCellError};
use tonlib_core::TonHash;
mod error;
mod ipfs_loader;
mod loader;

use std::fmt::Debug;

use async_trait::async_trait;
use lazy_static::lazy_static;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use tonlib_core::cell::dict::SnakeFormatDict;
use tonlib_core::types::ZERO_HASH;
struct MetaDataField {
    pub(crate) key: TonHash,
}

impl MetaDataField {
    fn new(name: &str) -> MetaDataField {
        let key = Self::key_from_str(name).unwrap_or(ZERO_HASH);
        MetaDataField { key }
    }

    fn key_from_str(k: &str) -> Result<TonHash, MetaLoaderError> {
        let mut hasher: Sha256 = Sha256::new();
        hasher.update(k);
        let slice = &hasher.finalize()[..];
        TryInto::<TonHash>::try_into(slice)
            .map_err(|e| MetaLoaderError::InternalError(e.to_string()))
    }

    pub fn use_string_or(&self, src: Option<String>, dict: &SnakeFormatDict) -> Option<String> {
        src.or(dict
            .get(&self.key)
            .cloned()
            .and_then(|vec| String::from_utf8(vec).ok()))
    }

    pub fn use_value_or(&self, src: Option<Value>, dict: &SnakeFormatDict) -> Option<Value> {
        src.or(dict
            .get(&self.key)
            .map(|attr_str| {
                Some(Value::Array(vec![Value::String(
                    String::from_utf8_lossy(attr_str).to_string().clone(),
                )]))
            })
            .unwrap_or_default())
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
    Internal { dict: SnakeFormatDict },
    // TODO: Replace with cell
    Unsupported { boc: BagOfCells },
}

impl MetaDataContent {
    pub fn parse(cell: &ArcCell) -> Result<MetaDataContent, TonCellError> {
        let res = match parse_impl(cell) {
            Ok(content) => content,
            Err(err) => {
                log::warn!("Fail to parse metadata: {err}");
                MetaDataContent::Unsupported {
                    boc: BagOfCells {
                        roots: vec![cell.clone()],
                    },
                }
            }
        };
        Ok(res)
    }
}

fn parse_impl(cell: &ArcCell) -> Result<MetaDataContent, TonCellError> {
    let mut parser = cell.parser();
    let content_repr = parser.load_byte()?;
    match content_repr {
        0 => {
            let dict = parser.load_dict_snake_format()?;
            Ok(MetaDataContent::Internal { dict })
        }
        1 => {
            let data = parser.load_snake_format_aligned(false)?;
            let uri = String::from_utf8(data).map_cell_parser_error()?;
            Ok(MetaDataContent::External { uri })
        }
        _ => Ok(MetaDataContent::Unsupported {
            boc: BagOfCells {
                roots: vec![cell.clone()],
            },
        }),
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

#[cfg(test)]
mod tests {
    use tonlib_core::cell::CellBuilder;

    use super::*;

    #[test]
    fn test_meta_snake_format_in_ref_cell() -> anyhow::Result<()> {
        let child = CellBuilder::new()
            .store_bits(440, &hex::decode("68747470733A2F2F676966746966792D6170702E70616C657474652E66696E616E63652F746F79626561722D6A6574746F6E2E6A736F6E")?)?
            .build()?;

        let meta_cell = CellBuilder::new()
            .store_byte(1)?
            .store_reference(&child.to_arc())?
            .build()?;

        let content = MetaDataContent::parse(&meta_cell.to_arc())?;
        assert_eq!(
            content,
            MetaDataContent::External {
                uri: "https://giftify-app.palette.finance/toybear-jetton.json".to_string()
            }
        );

        Ok(())
    }

    #[test]
    fn test_meta_unaligned() -> anyhow::Result<()> {
        let root = BagOfCells::parse_hex("b5ee9c7201010a0100d400010300c00102012002060143bff872ebdb514d9c97c283b7f0ae5179029e2b6119c39462719e4f46ed8f7413e6400301050000c00402012005060143bff872ebdb514d9c97c283b7f0ae5179029e2b6119c39462719e4f46ed8f7413e640070143bff7407e978f01a40711411b1acb773a96bdd93fa83bb5ca8435013c8c4b3ac91f400901020008008c68747470733a2f2f6a736f6e626c6f622e636f6d2f6170692f6a736f6e426c6f622f30313964666366342d333764322d376139332d393333392d31303131303138356365333000040039")?;
        let meta = MetaDataContent::parse(&root.single_root().unwrap())?;
        assert!(matches!(meta, MetaDataContent::Unsupported { .. }));
        Ok(())
    }
}
