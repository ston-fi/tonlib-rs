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
                if dict.contains_key(&META_URI.key) {
                    let uri = String::from_utf8_lossy(dict.get(&META_URI.key).unwrap()).to_string();
                    let external_meta = self.load_meta_from_uri(uri.as_str()).await?;
                    Ok(NftCollectionMetaData {
                        image: META_IMAGE.use_string_or(external_meta.image, dict),
                        name: META_NAME.use_string_or(external_meta.name, dict),
                        description: META_DESCRIPTION
                            .use_string_or(external_meta.description, dict),
                        social_links: META_SOCIAL_LINKS
                            .use_value_or(external_meta.social_links, dict),
                        marketplace: META_MARKETPLACE
                            .use_string_or(external_meta.marketplace, dict),
                    })
                } else {
                    Ok(NftCollectionMetaData {
                        image: META_IMAGE.use_string_or(None, dict),
                        name: META_NAME.use_string_or(None, dict),
                        description: META_DESCRIPTION.use_string_or(None, dict),
                        social_links: META_SOCIAL_LINKS.use_value_or(None, dict),
                        marketplace: META_MARKETPLACE.use_string_or(None, dict),
                    })
                }
            }
            content => Err(MetaLoaderError::ContentLayoutUnsupported(content.clone())),
        }
    }
}
