use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::meta::*;

#[derive(Serialize, PartialEq, Eq, Deserialize, Debug, Clone)]
pub struct NftCollectionMetaData {
    /// Optional. ASCII string. A URI pointing to a resource with mime type image.
    pub image: Option<String>,
    /// Optional. UTF8 string. Identifies the asset.
    pub name: Option<String>,
    /// Optional. UTF8 string. Describes the asset.
    pub description: Option<String>,
    /// Optional. No description in TEP64 yet
    pub social_links: Option<Value>,
    /// Optional. No description in TEP64 yet
    pub marketplace: Option<String>,
}

#[async_trait]
impl LoadMeta<NftCollectionMetaData> for MetaLoader<NftCollectionMetaData> {
    async fn load(
        &self,
        content: &MetaDataContent,
    ) -> Result<NftCollectionMetaData, MetaLoaderError> {
        match content {
            MetaDataContent::External { uri } => self.load_meta_from_uri(uri.as_str()).await,
            MetaDataContent::Internal { dict } => {
                if dict.contains_key("uri") {
                    let uri = dict.get(&META_URI.key).unwrap();
                    let external_meta = self.load_meta_from_uri(uri.as_str()).await?;
                    Ok(NftCollectionMetaData {
                        image: external_meta
                            .image
                            .or_else(|| dict.get(&META_IMAGE.key).cloned()),
                        name: external_meta
                            .name
                            .or_else(|| dict.get(&META_NAME.key).cloned()),
                        description: external_meta
                            .description
                            .or_else(|| dict.get(&META_DESCRIPTION.key).cloned()),
                        social_links: external_meta.social_links.or_else(|| {
                            dict.get(&META_SOCIAL_LINKS.key)
                                .map(|attr_str| {
                                    Some(Value::Array(vec![Value::String(attr_str.clone())]))
                                })
                                .unwrap_or_default()
                        }),

                        marketplace: external_meta
                            .marketplace
                            .or_else(|| dict.get(&META_MARKETPLACE.key).cloned()),
                    })
                } else {
                    Ok(NftCollectionMetaData {
                        image: dict.get(&META_IMAGE.key).cloned(),
                        name: dict.get(&META_NAME.key).cloned(),
                        description: dict.get(&META_DESCRIPTION.key).cloned(),
                        social_links: dict
                            .get(&META_SOCIAL_LINKS.key)
                            .map(|attr_str| {
                                Some(Value::Array(vec![Value::String(attr_str.clone())]))
                            })
                            .unwrap_or_default(),
                        marketplace: dict.get(&META_MARKETPLACE.key).cloned(),
                    })
                }
            }
            content => Err(MetaLoaderError::ContentLayoutUnsupported {
                content: content.clone(),
            }),
        }
    }
}
