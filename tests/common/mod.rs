use std::sync::Once;

use log::LevelFilter;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::config::{Appender, Root};
use log4rs::Config;

use tonlib::{
    client::{TonClient, TonConnectionParams},
    config::{MAINNET_CONFIG, TESTNET_CONFIG},
};

#[allow(dead_code)]
static LOG: Once = Once::new();

#[allow(dead_code)]
pub fn init_logging() {
    LOG.call_once(|| {
        TonClient::set_log_verbosity_level(2);
        let stderr = ConsoleAppender::builder()
            .target(Target::Stderr)
            .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
                "{d(%Y-%m-%d %H:%M:%S%.6f)} {T:>15.15} {h({l:>5.5})} {t}:{L} - {m}{n}",
            )))
            .build();

        let config = Config::builder()
            .appender(Appender::builder().build("stderr", Box::new(stderr)))
            .build(Root::builder().appender("stderr").build(LevelFilter::Trace))
            .unwrap();

        log4rs::init_config(config).unwrap();
    })
}

#[allow(dead_code)]
pub async fn new_testnet_client() -> anyhow::Result<TonClient> {
    let mut params = TonConnectionParams::default();
    params.config = TESTNET_CONFIG.to_string();
    let client = TonClient::builder()
        .with_connection_params(&params)
        .with_pool_size(2)
        .with_logging_callback()
        .with_keystore_dir("./var/ton/testnet".to_string())
        .build()
        .await?;
    Ok(client)
}

#[allow(dead_code)]
pub async fn new_archive_testnet_client() -> anyhow::Result<TonClient> {
    let mut params = TonConnectionParams::default();
    params.config = TESTNET_CONFIG.to_string();
    let client = TonClient::builder()
        .with_connection_params(&params)
        .with_pool_size(2)
        .with_logging_callback()
        .with_keystore_dir("./var/ton/testnet".to_string())
        .with_archive_nodes_only()
        .build()
        .await?;
    Ok(client)
}

#[allow(dead_code)]
pub async fn new_mainnet_client() -> anyhow::Result<TonClient> {
    let mut params = TonConnectionParams::default();
    params.config = MAINNET_CONFIG.to_string();
    let client = TonClient::builder()
        .with_connection_params(&params)
        .with_logging_callback()
        .with_keystore_dir("./var/ton".to_string())
        .build()
        .await?;
    Ok(client)
}

#[allow(dead_code)]
pub async fn new_archive_mainnet_client() -> anyhow::Result<TonClient> {
    let mut params = TonConnectionParams::default();
    params.config = MAINNET_CONFIG.to_string();
    let client = TonClient::builder()
        .with_connection_params(&params)
        .with_pool_size(2)
        .with_logging_callback()
        .with_keystore_dir("./var/ton".to_string())
        .with_archive_nodes_only()
        .build()
        .await?;
    Ok(client)
}
