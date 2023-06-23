use std::fmt::Debug;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum IpfsLoaderConfig {
    HttpGateway { url: String },
    IpfsNode { url: String },
}

impl IpfsLoaderConfig {
    pub fn http_gateway(url: &str) -> IpfsLoaderConfig {
        IpfsLoaderConfig::HttpGateway {
            url: url.to_string(),
        }
    }

    pub fn ipfs_node(url: &str) -> IpfsLoaderConfig {
        IpfsLoaderConfig::IpfsNode {
            url: url.to_string(),
        }
    }
}

impl Default for IpfsLoaderConfig {
    fn default() -> Self {
        Self::HttpGateway {
            url: "https://cloudflare-ipfs.com/ipfs/".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct IpfsLoader {
    config: IpfsLoaderConfig,
    client: reqwest::Client,
}

impl IpfsLoader {
    pub fn new(config: &IpfsLoaderConfig) -> anyhow::Result<Self> {
        Ok(Self {
            config: config.clone(),
            client: reqwest::Client::builder().build()?,
        })
    }

    pub async fn load(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        let response = match &self.config {
            IpfsLoaderConfig::HttpGateway { url } => {
                let uri = format!("{}/{}", url, path);
                self.client.get(uri).send().await?
            }
            IpfsLoaderConfig::IpfsNode { url } => {
                let uri = format!("{}/api/v0/cat?arg={}", url, path);
                self.client.post(uri).send().await?
            }
        };
        let status = response.status();
        if status.is_success() {
            let bytes = response.bytes().await?.to_vec();
            Ok(bytes)
        } else {
            const MAX_MESSAGE_SIZE: usize = 200;
            let body = String::from_utf8(response.bytes().await?.to_vec())?;
            let message = if body.len() > MAX_MESSAGE_SIZE {
                format!("{}...", &body[0..MAX_MESSAGE_SIZE - 3])
            } else {
                body.clone()
            };
            anyhow::bail!(
                "Failed to load IPFS object {}, status: {}, message: {}",
                path,
                status,
                message
            );
        }
    }

    pub async fn load_utf8(&self, path: &str) -> anyhow::Result<String> {
        let bytes = self.load(path).await?;
        let str = String::from_utf8(bytes)?;
        Ok(str)
    }

    pub fn default() -> anyhow::Result<Self> {
        Self::new(&IpfsLoaderConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use crate::ipfs::IpfsLoaderConfig;

    static CONFIG_JSON: &str = r#"
    {
      "http_gateway": {
        "url": "http://example.com/"
      }
    }
    "#;

    #[test]
    fn test_config_deserialization() -> anyhow::Result<()> {
        let config: IpfsLoaderConfig = serde_json::from_str(CONFIG_JSON)?;
        match config {
            IpfsLoaderConfig::HttpGateway { url } => {
                assert_eq!(url, "http://example.com/");
            }
            _ => {
                panic!("Expected HttpGateway config, got {:?}", config);
            }
        };
        Ok(())
    }
}
