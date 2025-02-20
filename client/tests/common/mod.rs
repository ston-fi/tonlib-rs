use std::sync::Once;

use log::LevelFilter;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::config::{Appender, Root};
use log4rs::Config;
use tokio_test::assert_ok;
use tonlib_client::client::{ConnectionCheck, TonClient, TonConnectionParams};
use tonlib_client::config::{MAINNET_CONFIG, TESTNET_CONFIG};
use tonlib_client::contract::TonContractFactory;

#[allow(dead_code)]
static LOG: Once = Once::new();

#[allow(dead_code)]
pub fn init_logging() {
    LOG.call_once(|| {
        TonClient::set_log_verbosity_level(1);
        let stderr = ConsoleAppender::builder()
            .target(Target::Stderr)
            .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
                "{d(%Y-%m-%d %H:%M:%S%.6f)} {T:>15.15} {h({l:>5.5})} {t}:{L} - {m}{n}",
            )))
            .build();

        let config = Config::builder()
            .appender(Appender::builder().build("stderr", Box::new(stderr)))
            .build(Root::builder().appender("stderr").build(LevelFilter::Info))
            .unwrap();

        log4rs::init_config(config).unwrap();
    })
}

#[allow(dead_code)]
pub async fn new_mainnet_client() -> TonClient {
    assert_ok!(new_ton_client(false, false).await)
}

#[allow(dead_code)]
pub async fn new_mainnet_client_archive() -> TonClient {
    assert_ok!(new_ton_client(false, true).await)
}

#[allow(dead_code)]
pub async fn new_testnet_client() -> TonClient {
    assert_ok!(new_ton_client(true, false).await)
}

#[allow(dead_code)]
pub async fn new_testnet_client_archive() -> TonClient {
    assert_ok!(new_ton_client(true, true).await)
}

#[allow(dead_code)]
pub async fn new_contract_factory(
    testnet: bool,
    archive: bool,
) -> anyhow::Result<TonContractFactory> {
    let ton_cli = new_ton_client(testnet, archive).await?;
    Ok(TonContractFactory::builder(&ton_cli).build().await?)
}

#[allow(dead_code)]
pub async fn new_ton_client(testnet: bool, archive: bool) -> anyhow::Result<TonClient> {
    init_logging();
    let (ton_config, keystore) = match testnet {
        true => (TESTNET_CONFIG.to_string(), "./var/ton/testnet"),
        false => (MAINNET_CONFIG.to_string(), "./var/ton"),
    };

    let conn_params = TonConnectionParams {
        config: ton_config,
        ..Default::default()
    };

    let mut builder = TonClient::builder();
    builder
        .with_connection_params(&conn_params)
        .with_pool_size(2)
        .with_logging_callback()
        .with_keystore_dir(keystore.to_string());

    match archive {
        true => builder.with_connection_check(ConnectionCheck::Archive),
        false => builder.with_connection_check(ConnectionCheck::Health),
    };
    Ok(builder.build().await?)
}
