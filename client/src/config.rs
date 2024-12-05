use std::path::Path;
use std::{env, fs};

use serde::{Deserialize, Serialize};
use serde_json::Value;

lazy_static::lazy_static! {
    pub static ref MAINNET_CONFIG: String = load_config(
        "TONLIB_MAINNET_CONF",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/config/global.config.json"))
    );

    pub static ref TESTNET_CONFIG: String = load_config(
        "TONLIB_TESTNET_CONF",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/config/testnet-global.config.json"))
    );
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TonConfig {
    #[serde(rename = "@type")]
    conf_type: Value,
    dht: Value,
    pub liteservers: Vec<LiteEndpoint>,
    validator: Validator,
}

#[cfg(feature = "liteapi")]
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

    pub fn set_init_block(
        &mut self,
        block_id: &crate::tl::BlockIdExt,
    ) -> Result<(), serde_json::Error> {
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

fn read_config_file(path: &Path) -> String {
    match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => {
            log::error!("Failed to read configuration file {:?}: {}", path, err);
            "".to_string()
        }
    }
}

/// Loads the configuration file.
/// - `env_var`: The name of the environment variable to check for a custom path.
/// - `default_config`: The default configuration embedded at compile time.
fn load_config(env_var: &str, default_config: &'static str) -> String {
    if let Ok(custom_path) = env::var(env_var) {
        let config_path = Path::new(&custom_path);
        match fs::canonicalize(config_path) {
            Ok(absolute_path) => {
                log::info!(
                    "Using custom config for {} from: {:?}",
                    env_var,
                    absolute_path
                );
                read_config_file(&absolute_path)
            }
            Err(err) => {
                log::error!(
                    "Failed to resolve path for {:?} {:?}: {}",
                    env_var,
                    config_path,
                    err
                );
                default_config.to_string()
            }
        }
    } else {
        log::info!(
            "Using default config for {} embedded at compile time",
            env_var
        );
        default_config.to_string()
    }
}
