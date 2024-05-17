use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::meta::*;

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
impl LoadMeta<NftItemMetaData> for MetaLoader<NftItemMetaData> {
    async fn load(&self, content: &MetaDataContent) -> Result<NftItemMetaData, MetaLoaderError> {
        match content {
            MetaDataContent::External { uri } => self.load_meta_from_uri(uri.as_str()).await,
            MetaDataContent::Internal { dict } => {
                if dict.contains_key(&META_URI.key) {
                    let uri = String::from_utf8_lossy(dict.get(&META_URI.key).unwrap()).to_string();
                    let external_meta = self.load_meta_from_uri(uri.as_str()).await?;
                    Ok(NftItemMetaData {
                        name: META_NAME.use_string_or(external_meta.name, dict),
                        content_url: META_URI.use_string_or(external_meta.content_url, dict),
                        description: META_DESCRIPTION
                            .use_string_or(external_meta.description, dict),
                        image: META_IMAGE.use_string_or(external_meta.image, dict),
                        attributes: META_ATTRIBUTES.use_value_or(external_meta.attributes, dict),
                    })
                } else {
                    Ok(NftItemMetaData {
                        name: META_NAME.use_string_or(None, dict),
                        content_url: META_URI.use_string_or(None, dict),
                        description: META_DESCRIPTION.use_string_or(None, dict),
                        image: META_IMAGE.use_string_or(None, dict),
                        attributes: META_ATTRIBUTES.use_value_or(None, dict),
                    })
                }
            }
            content => Err(MetaLoaderError::ContentLayoutUnsupported(content.clone())),
        }
    }
}
