use futures::TryStreamExt;
use ipfs_api_backend_hyper::{IpfsApi, IpfsClient, TryFromUri};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(tag = "@type")]
pub enum IpfsLoaderConfig {
    HttpGateway { uri: String },
    IpfsNode { uri: String },
}

impl IpfsLoaderConfig {
    pub fn http_gateway(uri: &str) -> IpfsLoaderConfig {
        IpfsLoaderConfig::HttpGateway {
            uri: uri.to_string(),
        }
    }

    pub fn ipfs_node(uri: &str) -> IpfsLoaderConfig {
        IpfsLoaderConfig::IpfsNode {
            uri: uri.to_string(),
        }
    }
}

impl Default for IpfsLoaderConfig {
    fn default() -> Self {
        Self::HttpGateway {
            uri: "https://cloudflare-ipfs.com/ipfs/".to_string(),
        }
    }
}

pub struct IpfsLoader {
    backend: IpfsLoaderBackend,
}

impl IpfsLoader {
    pub fn new(config: &IpfsLoaderConfig) -> anyhow::Result<Self> {
        let loader: IpfsLoaderBackend = match config {
            IpfsLoaderConfig::HttpGateway { uri } => IpfsLoaderBackend::HttpGateway {
                prefix: uri.clone(),
                client: reqwest::Client::builder().build()?,
            },
            IpfsLoaderConfig::IpfsNode { uri } => IpfsLoaderBackend::IpfsNode {
                client: IpfsClient::from_str(uri.as_str())?,
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
