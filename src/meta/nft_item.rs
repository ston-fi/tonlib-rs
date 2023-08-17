use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::meta::*;
use anyhow::anyhow;
use serde_json::{self, Value};

#[derive(Serialize, PartialEq, Eq, Deserialize, Debug, Clone)]
pub struct NftItemMetaData {
    ///  Optional. UTF8 string. Identifies the asset.
    pub name: Option<String>,
    /// Optional. UTF8 string. Describes the asset.
    pub description: Option<String>,
    /// Optional. ASCII string. A URI pointing to a resource with mime type image.
    pub image: Option<String>,
    /// Optional. No description in TEP64 yet
    pub content_url: Option<String>,
    /// Optional. No description in TEP64 yet
    pub attributes: Option<Value>,
}

#[async_trait]
impl LoadMeta<NftItemMetaData> for MetaLoader<'_, NftItemMetaData> {
    async fn load(&self, content: MetaDataContent) -> anyhow::Result<NftItemMetaData> {
        match content {
            MetaDataContent::External { uri } => self.load_meta_from_uri(uri.as_str()).await,
            MetaDataContent::Internal { dict } => {
                if dict.contains_key("uri") {
                    let uri = dict.get(&META_URI.key).unwrap();
                    let external_meta = self.load_meta_from_uri(uri.as_str()).await?;
                    Ok(NftItemMetaData {
                        name: external_meta
                            .name
                            .or_else(|| dict.get(&META_NAME.key).cloned()),
                        content_url: external_meta
                            .content_url
                            .or_else(|| dict.get(&META_URI.key).cloned()),
                        description: external_meta
                            .description
                            .or_else(|| dict.get(&META_DESCRIPTION.key).cloned()),
                        image: external_meta
                            .image
                            .or_else(|| dict.get(&META_IMAGE.key).cloned()),
                        attributes: external_meta.attributes.or_else(|| {
                            dict.get(&META_ATTRIBUTES.key)
                                .map(|attr_str| {
                                    Some(Value::Array(vec![Value::String(attr_str.clone())]))
                                })
                                .unwrap_or_default()
                        }),
                    })
                } else {
                    Ok(NftItemMetaData {
                        name: dict.get(&META_NAME.key).cloned(),
                        content_url: dict.get(&META_CONTENT_URL.key).cloned(),
                        description: dict.get(&META_DESCRIPTION.key).cloned(),
                        image: dict.get(&META_IMAGE.key).cloned(),
                        attributes: dict
                            .get(&META_ATTRIBUTES.key)
                            .map(|attr_str| {
                                Some(Value::Array(vec![Value::String(attr_str.clone())]))
                            })
                            .unwrap_or_default(),
                    })
                }
            }
            other => Err(anyhow!("Unsupported content layout {:?}", other)),
        }
    }
}
