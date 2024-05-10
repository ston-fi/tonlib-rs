use std::path::Path;
use std::sync::Once;
use std::{env, fs};

use lazy_static::lazy_static;
use log::LevelFilter;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::config::{Appender, Root};
use log4rs::Config;
use tokio_test::assert_ok;
use tonlib::client::{ConnectionCheck, TonClient, TonConnectionParams};
use tonlib::config::TESTNET_CONFIG;

lazy_static! {
    pub static ref MAINNET_CONFIG: &'static str = {
        let maybe_local_config_flag = env::var("USE_LOCAL_TON_MAINNET_CONFIG");
        let local_config_flag = match maybe_local_config_flag {
            Ok(flag) => flag.parse::<bool>().unwrap_or(false),
            Err(_) => false,
        };

        let config_path = if local_config_flag {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/config/local/local.config.json")
        } else {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/config/global.config.json")
        };

        read_config_file(&config_path)
    };
}

fn read_config_file(path: &Path) -> &'static str {
    match fs::read_to_string(path) {
        Ok(content) => Box::leak(content.into_boxed_str()),
        Err(err) => {
            eprintln!("Error reading config file {:?}: {}", path, err);
            ""
        }
    }
}

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
            .build(Root::builder().appender("stderr").build(LevelFilter::Trace))
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
