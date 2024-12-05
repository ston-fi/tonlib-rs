use std::sync::Once;

use log::LevelFilter;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::config::{Appender, Root};
use log4rs::Config;
use tokio_test::assert_ok;
use tonlib_client::client::{ConnectionCheck, TonClient, TonConnectionParams};
use tonlib_client::config::{MAINNET_CONFIG, TESTNET_CONFIG};

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
pub async fn new_testnet_client() -> TonClient {
    let params = TonConnectionParams {
        config: TESTNET_CONFIG.to_string(),
        ..Default::default()
    };
    assert_ok!(
        TonClient::builder()
            .with_connection_params(&params)
            .with_pool_size(2)
            .with_logging_callback()
            .with_keystore_dir("./var/ton/testnet".to_string())
            .build()
            .await
    )
}

#[allow(dead_code)]
pub async fn new_archive_testnet_client() -> TonClient {
    let params = TonConnectionParams {
        config: TESTNET_CONFIG.to_string(),
        ..Default::default()
    };
    assert_ok!(
        TonClient::builder()
            .with_connection_params(&params)
            .with_pool_size(2)
            .with_logging_callback()
            .with_keystore_dir("./var/ton/testnet".to_string())
            .with_connection_check(ConnectionCheck::Archive)
            .build()
            .await
    )
}

#[allow(dead_code)]
pub async fn new_mainnet_client() -> TonClient {
    let params = TonConnectionParams {
        config: MAINNET_CONFIG.to_string(),
        ..Default::default()
    };
    assert_ok!(
        TonClient::builder()
            .with_connection_params(&params)
            .with_pool_size(2)
            .with_logging_callback()
            .with_keystore_dir("./var/ton".to_string())
            .with_connection_check(ConnectionCheck::Health)
            .build()
            .await
    )
}

#[allow(dead_code)]
pub async fn new_archive_mainnet_client() -> TonClient {
    let params = TonConnectionParams {
        config: MAINNET_CONFIG.to_string(),
        ..Default::default()
    };

    assert_ok!(
        TonClient::builder()
            .with_connection_params(&params)
            .with_pool_size(2)
            .with_logging_callback()
            .with_keystore_dir("./var/ton".to_string())
            .with_connection_check(ConnectionCheck::Archive)
            .build()
            .await
    )
}
