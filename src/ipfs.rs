use std::fmt::Debug;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum IpfsConnectionType {
    HttpGateway,
    IpfsNode,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub struct IpfsLoaderConfig {
    connection_type: IpfsConnectionType,
    base_url: String,
}

impl IpfsLoaderConfig {
    pub fn http_gateway(url: &str) -> IpfsLoaderConfig {
        IpfsLoaderConfig {
            connection_type: IpfsConnectionType::HttpGateway,
            base_url: url.to_string(),
        }
    }

    pub fn ipfs_node(url: &str) -> IpfsLoaderConfig {
        IpfsLoaderConfig {
            connection_type: IpfsConnectionType::IpfsNode,
            base_url: url.to_string(),
        }
    }
}

impl Default for IpfsLoaderConfig {
    fn default() -> Self {
        Self {
            connection_type: IpfsConnectionType::HttpGateway,
            base_url: "https://cloudflare-ipfs.com/ipfs/".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct IpfsLoader {
    connection_type: IpfsConnectionType,
    base_url: String,
    client: reqwest::Client,
}

impl IpfsLoader {
    pub fn new(config: &IpfsLoaderConfig) -> anyhow::Result<Self> {
        Ok(Self {
            connection_type: config.connection_type.clone(),
            base_url: config.base_url.clone(),
            client: reqwest::Client::builder().build()?,
        })
    }

    pub fn default() -> anyhow::Result<Self> {
        Self::new(&IpfsLoaderConfig::default())
    }

    pub async fn load(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        let response = match self.connection_type {
            IpfsConnectionType::HttpGateway => {
                let full_url = format!("{}/{}", self.base_url, path);
                self.client.get(full_url).send().await?
            }
            IpfsConnectionType::IpfsNode => {
                let full_url = format!("{}/api/v0/cat?arg={}", self.base_url, path);
                self.client.post(full_url).send().await?
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
}

#[cfg(test)]
mod tests {
    use crate::ipfs::{IpfsConnectionType, IpfsLoaderConfig};

    static CONFIG_JSON: &str = r#"
    {
      "connection_type": "http_gateway",
      "base_url": "http://example.com/"
    }
    "#;

    #[test]
    fn test_config_deserialization() -> anyhow::Result<()> {
        let config: IpfsLoaderConfig = serde_json::from_str(CONFIG_JSON)?;
        assert_eq!(config.connection_type, IpfsConnectionType::HttpGateway);
        assert_eq!(config.base_url, "http://example.com/");
        Ok(())
    }
}
