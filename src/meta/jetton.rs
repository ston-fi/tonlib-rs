use anyhow::anyhow;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_aux::prelude::*;

use crate::meta::*;

#[derive(Serialize, PartialEq, Eq, Deserialize, Debug, Clone)]
pub struct JettonMetaData {
    ///Optional. UTF8 string. The name of the token - e.g. "Example Coin".
    pub name: Option<String>,
    ///Optional. Used by "Semi-chain content layout". ASCII string. A URI pointing to JSON document with metadata.
    pub uri: Option<String>,
    ///Optional. UTF8 string. The symbol of the token - e.g. "XMPL". Used in the form "You received 99 XMPL".
    pub symbol: Option<String>,
    ///Optional. UTF8 string. Describes the token - e.g. "This is an example jetton for the TON network".
    pub description: Option<String>,
    ///Optional. ASCII string. A URI pointing to a jetton icon with mime type image.
    pub image: Option<String>,
    ///Optional. Either binary representation of the image for onchain layout or base64 for offchain layout.
    pub image_data: Option<String>,
    ///Optional. If not specified, 9 is used by default. UTF8 encoded string with number from 0 to 255.
    ///The number of decimals the token uses - e.g. 8, means to divide the token amount by 100000000
    ///to get its user representation, while 0 means that tokens are indivisible:
    ///user representation of token number should correspond to token amount in wallet-contract storage.
    ///
    ///In case you specify decimals, it is highly recommended that you specify this parameter
    ///on-chain and that the smart contract code ensures that this parameter is immutable.
    #[serde(default, deserialize_with = "deserialize_option_number_from_string")]
    pub decimals: Option<u8>,
}

#[async_trait]
impl LoadMeta<JettonMetaData> for MetaLoader<JettonMetaData> {
    async fn load(&self, content: &MetaDataContent) -> anyhow::Result<JettonMetaData> {
        match content {
            MetaDataContent::External { uri } => self.load_meta_from_uri(uri.as_str()).await,
            MetaDataContent::Internal { dict } => {
                if dict.contains_key(&META_URI.key) {
                    let uri = dict.get(&META_URI.key).unwrap();
                    let external_meta = self.load_meta_from_uri(uri.as_str()).await?;
                    Ok(JettonMetaData {
                        name: external_meta.name.or(dict.get(&META_NAME.key).cloned()),
                        uri: external_meta.uri.or(dict.get(&META_URI.key).cloned()),
                        symbol: external_meta.symbol.or(dict.get(&META_SYMBOL.key).cloned()),
                        description: external_meta
                            .description
                            .or(dict.get(&META_DESCRIPTION.key).cloned()),
                        image: external_meta.image.or(dict.get(&META_IMAGE.key).cloned()),
                        image_data: external_meta
                            .image_data
                            .or(dict.get(&META_IMAGE_DATA.key).cloned()),
                        decimals: external_meta.decimals.or(dict
                            .get(&META_DECIMALS.key)
                            .and_then(|v| v.parse::<u8>().ok())),
                    })
                } else {
                    Ok(JettonMetaData {
                        name: dict.get(&META_NAME.key).cloned(),
                        uri: dict.get(&META_URI.key).cloned(),
                        symbol: dict.get(&META_SYMBOL.key).cloned(),
                        description: dict.get(&META_DESCRIPTION.key).cloned(),
                        image: dict.get(&META_IMAGE.key).cloned(),
                        image_data: dict.get(&META_IMAGE_DATA.key).cloned(),
                        decimals: dict
                            .get(&META_DECIMALS.key)
                            .and_then(|v| v.parse::<u8>().ok()),
                    })
                }
            }
            other => Err(anyhow!("Unsupported content layout {:?}", other)),
        }
    }
}
