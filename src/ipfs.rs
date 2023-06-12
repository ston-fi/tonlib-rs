use futures::TryStreamExt;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient, TryFromUri};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

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

pub struct IpfsLoader {
    backend: IpfsLoaderBackend,
}

impl IpfsLoader {
    pub fn new(config: &IpfsLoaderConfig) -> anyhow::Result<Self> {
        let loader: IpfsLoaderBackend = match config {
            IpfsLoaderConfig::HttpGateway { url } => IpfsLoaderBackend::HttpGateway {
                prefix: url.clone(),
                client: reqwest::Client::builder().build()?,
            },
            IpfsLoaderConfig::IpfsNode { url } => IpfsLoaderBackend::IpfsNode {
                client: IpfsClient::from_str(url.as_str())?,
            },
        };
        Ok(Self { backend: loader })
    }

    pub async fn load(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        self.backend.load(path).await
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

enum IpfsLoaderBackend {
    HttpGateway {
        prefix: String,
        client: reqwest::Client,
    },
    IpfsNode {
        client: IpfsClient,
    },
}

impl IpfsLoaderBackend {
    pub async fn load(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        match self {
            IpfsLoaderBackend::HttpGateway { prefix, client } => {
                let url = format!("{}{}", prefix, path);
                let resp = client.get(url).send().await?;
                let bytes = if resp.status().is_success() {
                    resp.bytes().await?.to_vec()
                } else {
                    anyhow::bail!(
                        "Failed to load IPFS object {}, status: {}",
                        path,
                        resp.status()
                    );
                };
                Ok(bytes)
            }
            IpfsLoaderBackend::IpfsNode { client } => {
                let bytes = client
                    .cat(path)
                    .map_ok(|chunk| chunk.to_vec())
                    .try_concat()
                    .await?;
                Ok(bytes)
            }
        }
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
