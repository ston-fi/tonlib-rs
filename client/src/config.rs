use crate::tl::BlockIdExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MAINNET_CONFIG: &str = include_str!("../resources/config/global.config.json");
pub const TESTNET_CONFIG: &str = include_str!("../resources/config/testnet-global.config.json");

#[derive(Serialize, Deserialize)]
pub(crate) struct TonConfig {
    #[serde(rename = "@type")]
    conf_type: Value,
    dht: Value,
    pub liteservers: Vec<LiteEndpoint>,
    validator: Validator,
}

impl TonConfig {
    pub fn from_json(config: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(config)
    }
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn get_init_block_seqno(&self) -> i32 {
        self.validator.init_block["seqno"].as_i64().unwrap_or(0) as i32
    }

    pub fn set_init_block(&mut self, block_id: &BlockIdExt) -> Result<(), serde_json::Error> {
        self.validator.init_block = serde_json::to_value(block_id)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LiteEndpoint {
    pub ip: i32,
    pub port: u16,
    pub id: LiteID,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LiteID {
    #[serde(rename = "@type")]
    pub config_type: Value,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Validator {
    #[serde(rename = "@type")]
    pub config_type: Value,
    pub zero_state: Value,
    pub init_block: Value,
    pub hardforks: Value,
}
