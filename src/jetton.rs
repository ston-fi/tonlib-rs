use std::collections::HashMap;
use std::str;

use anyhow::anyhow;
use async_trait::async_trait;
use lazy_static::lazy_static;
use num_bigint::{BigInt, BigUint};
use num_traits::Num;
use serde::{Deserialize, Serialize};
use serde_aux::prelude::*;
use sha2::{Digest, Sha256};

use crate::address::TonAddress;
use crate::cell::{BagOfCells, CellBuilder};
use crate::contract::TonContract;
use crate::tl::stack::TvmSlice;
use crate::tl::stack::TvmStackEntry::Slice;

// Constants from jetton reference implementation:
// https://github.com/ton-blockchain/token-contract/blob/main/ft/op-codes.fc
pub const JETTON_TRANSFER: u32 = 0xf8a7ea5;
pub const JETTON_TRANSFER_NOTIFICATION: u32 = 0x7362d09c;
pub const JETTON_INTERNAL_TRANSFER: u32 = 0x178d4519;
pub const JETTON_EXCESSES: u32 = 0xd53276db;
pub const JETTON_BURN: u32 = 0x595f07bc;
pub const JETTON_BURN_NOTIFICATION: u32 = 0x7bdd97de;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct JettonData {
    pub total_supply: BigUint,
    pub mintable: bool,
    pub admin_address: TonAddress,
    pub content: JettonContent,
    pub wallet_code: BagOfCells,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum JettonContent {
    External { uri: String },
    Internal { dict: HashMap<String, String> },
    Unsupported { boc: BagOfCells },
}
lazy_static! {
    static ref JETTON_META_NAME: JettonMetaDataField = JettonMetaDataField::new("name");
    static ref JETTON_META_DESCRIPTION: JettonMetaDataField =
        JettonMetaDataField::new("description");
    static ref JETTON_META_IMAGE: JettonMetaDataField = JettonMetaDataField::new("image");
    static ref JETTON_META_SYMBOL: JettonMetaDataField = JettonMetaDataField::new("symbol");
    static ref JETTON_META_IMAGE_DATA: JettonMetaDataField = JettonMetaDataField::new("image_data");
    static ref JETTON_META_DECIMALS: JettonMetaDataField = JettonMetaDataField::new("decimals");
    static ref JETTON_META_URI: JettonMetaDataField = JettonMetaDataField::new("uri");
}
struct JettonMetaDataField {
    key: String,
}

impl JettonMetaDataField {
    fn new(name: &str) -> JettonMetaDataField {
        JettonMetaDataField {
            key: Self::key_from_str(name),
        }
    }

    fn key_from_str(k: &str) -> String {
        let mut hasher: Sha256 = Sha256::new();
        hasher.update(k);
        let s = hex::encode(hasher.finalize()[..].to_vec());
        BigInt::from_str_radix(s.as_str(), 16)
            .unwrap()
            .to_str_radix(10)
    }
}

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

pub struct JettonContentLoader {
    client: reqwest::Client,
}

impl JettonContentLoader {
    pub fn new() -> anyhow::Result<JettonContentLoader> {
        let client = reqwest::Client::builder().build()?;
        Ok(JettonContentLoader { client })
    }

    pub async fn load(&self, content: &JettonContent) -> anyhow::Result<JettonMetaData> {
        match content {
            JettonContent::External { uri } => self.load_meta_from_uri(uri).await,
            JettonContent::Internal { dict } => {
                if dict.contains_key(&JETTON_META_URI.key) {
                    let uri = dict.get(&JETTON_META_URI.key).unwrap();
                    let external_meta = self.load_meta_from_uri(uri).await?;
                    Ok(JettonMetaData {
                        name: external_meta
                            .name
                            .or(dict.get(&JETTON_META_NAME.key).cloned()),
                        uri: external_meta
                            .uri
                            .or(dict.get(&JETTON_META_URI.key).cloned()),
                        symbol: external_meta
                            .symbol
                            .or(dict.get(&JETTON_META_SYMBOL.key).cloned()),
                        description: external_meta
                            .description
                            .or(dict.get(&JETTON_META_DESCRIPTION.key).cloned()),
                        image: external_meta
                            .image
                            .or(dict.get(&JETTON_META_IMAGE.key).cloned()),
                        image_data: external_meta
                            .image_data
                            .or(dict.get(&JETTON_META_IMAGE_DATA.key).cloned()),
                        decimals: external_meta.decimals.or(dict
                            .get(&JETTON_META_DECIMALS.key)
                            .and_then(|v| v.parse::<u8>().ok())),
                    })
                } else {
                    Ok(JettonMetaData {
                        name: dict.get(&JETTON_META_NAME.key).cloned(),
                        uri: dict.get(&JETTON_META_URI.key).cloned(),
                        symbol: dict.get(&JETTON_META_SYMBOL.key).cloned(),
                        description: dict.get(&JETTON_META_DESCRIPTION.key).cloned(),
                        image: dict.get(&JETTON_META_IMAGE.key).cloned(),
                        image_data: dict.get(&JETTON_META_IMAGE_DATA.key).cloned(),
                        decimals: dict
                            .get(&JETTON_META_DECIMALS.key)
                            .and_then(|v| v.parse::<u8>().ok()),
                    })
                }
            }
            other => Err(anyhow!("Unsupported content layout {:?}", other)),
        }
    }

    async fn load_meta_from_uri(&self, uri: &String) -> anyhow::Result<JettonMetaData> {
        let url = if uri.starts_with("ipfs://") {
            let ipfs_token: String = uri.chars().into_iter().skip(7).collect();
            format!("{}{}", "https://cloudflare-ipfs.com/ipfs/", ipfs_token)
        } else {
            uri.clone()
        };
        let resp = self.client.get(url).send().await?;
        let resp_status = resp.status();
        if resp_status.is_success() {
            let text = resp.text().await?;
            let meta: JettonMetaData = serde_json::from_str(&text)?;
            Ok(meta)
        } else {
            anyhow::bail!(
                "Failed to load jetton metadata from {}. Resp status: {}",
                uri,
                resp_status
            );
        }
    }
}

#[async_trait]
pub trait JettonMasterContract {
    async fn get_jetton_data(&self) -> anyhow::Result<JettonData>;
    async fn get_wallet_address(&self, owner_address: &TonAddress) -> anyhow::Result<TonAddress>;
}

#[async_trait]
impl JettonMasterContract for TonContract {
    async fn get_jetton_data(&self) -> anyhow::Result<JettonData> {
        let res = self.run_get_method("get_jetton_data", &Vec::new()).await?;
        let stack = res.stack;
        if stack.elements.len() != 5 {
            Err(anyhow!(
                "Invalid get_jetton_data result from {}, expected 5 elements, got {}",
                self.address(),
                stack.elements.len()
            ))
        } else {
            let result = JettonData {
                total_supply: stack.get_biguint(0)?,
                mintable: stack.get_i32(1)? != 0,
                admin_address: stack
                    .get_boc(2)?
                    .single_root()?
                    .parse_fully(|r| r.load_address())?,
                content: read_jetton_content(&stack.get_boc(3)?)?,
                wallet_code: stack.get_boc(4)?,
            };
            Ok(result)
        }
    }

    async fn get_wallet_address(&self, owner_address: &TonAddress) -> anyhow::Result<TonAddress> {
        let cell = CellBuilder::new().store_address(owner_address)?.build()?;
        let boc = BagOfCells::from_root(cell);
        let slice = Slice {
            slice: TvmSlice {
                bytes: boc.serialize(true)?,
            },
        };

        let res = self
            .run_get_method("get_wallet_address", &vec![slice])
            .await?;
        let stack = res.stack;
        if stack.elements.len() != 1 {
            Err(anyhow!(
                "Invalid get_wallet_address result from {}, expected 1 element, got {}",
                self.address(),
                stack.elements.len()
            ))
        } else {
            let result = stack
                .get_boc(0)?
                .single_root()?
                .parse_fully(|r| r.load_address())?;
            Ok(result)
        }
    }
}

fn read_jetton_content(boc: &BagOfCells) -> anyhow::Result<JettonContent> {
    if let Ok(root) = boc.single_root() {
        let mut reader = root.parser();
        let tp = reader.load_byte()?;
        match tp {
            0 => {
                let dict = root.reference(0)?.load_snake_formatted_dict()?;
                Ok(JettonContent::Internal { dict })
            }
            1 => {
                let uri = reader.load_string(reader.remaining_bytes())?;
                Ok(JettonContent::External { uri })
            }
            _ => Ok(JettonContent::Unsupported { boc: boc.clone() }),
        }
    } else {
        Ok(JettonContent::Unsupported { boc: boc.clone() })
    }
}

#[derive(Debug, Clone)]
pub struct WalletData {
    pub balance: BigUint,
    pub owner_address: TonAddress,
    pub master_address: TonAddress,
    pub wallet_code: BagOfCells,
}

#[async_trait]
pub trait JettonWalletContract {
    async fn get_wallet_data(&self) -> anyhow::Result<WalletData>;
}

#[async_trait]
impl JettonWalletContract for TonContract {
    async fn get_wallet_data(&self) -> anyhow::Result<WalletData> {
        let res = self.run_get_method("get_wallet_data", &Vec::new()).await?;
        let stack = res.stack;
        if stack.elements.len() != 4 {
            Err(anyhow!(
                "Invalid get_wallet_data result from {}, expected 4 elements, got {}",
                self.address(),
                stack.elements.len()
            ))
        } else {
            let result = WalletData {
                balance: stack.get_biguint(0)?,
                owner_address: stack
                    .get_boc(1)?
                    .single_root()?
                    .parse_fully(|r| r.load_address())?,
                master_address: stack
                    .get_boc(2)?
                    .single_root()?
                    .parse_fully(|r| r.load_address())?,
                wallet_code: stack.get_boc(3)?,
            };
            Ok(result)
        }
    }
}
