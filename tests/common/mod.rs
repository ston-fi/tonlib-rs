use lazy_static::lazy_static;
use std::sync::{Arc, Once};
use std::time::Duration;

use log::LevelFilter;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::config::{Appender, Root};
use log4rs::Config;
use tonlib::client::{TonClient, TonConnectionCallback};
use tonlib::tl::{TonNotification, TonResult};

static LOG: Once = Once::new();

lazy_static! {
    pub static ref TEST_TON_CONNECTION_CALLBACK: Arc<TestTonConnectionCallback> =
        Arc::new(TestTonConnectionCallback {});
}

pub fn init_logging() {
    LOG.call_once(|| {
        TonClient::set_log_verbosity_level(2);
        let stderr = ConsoleAppender::builder().target(Target::Stderr).build();

        let config = Config::builder()
            .appender(Appender::builder().build("stderr", Box::new(stderr)))
            .build(Root::builder().appender("stderr").build(LevelFilter::Trace))
            .unwrap();

        log4rs::init_config(config).unwrap();
    })
}

#[allow(dead_code)]
pub async fn new_test_client() -> anyhow::Result<TonClient> {
    let client = TonClient::builder()
        .with_pool_size(2)
        .with_callback(TEST_TON_CONNECTION_CALLBACK.clone())
        .with_keystore_dir("./var/ton".to_string())
        .build()
        .await?;
    Ok(client)
}

pub struct TestTonConnectionCallback {}

impl TonConnectionCallback for TestTonConnectionCallback {
    fn on_invoke(&self, id: u32) {
        log::trace!("on_invoke: {:?}", id);
    }

    fn on_invoke_result(
        &self,
        id: u32,
        method: &str,
        duration: &Duration,
        res: &anyhow::Result<TonResult>,
    ) {
        log::trace!(
            "on_invoke_result: {:?} {} {} {:?}",
            id,
            method,
            duration.as_micros(),
            res
        );
    }

    fn on_notification(&self, notification: &TonNotification) {
        log::trace!("on_notification: {:?}", notification);
    }

    fn on_tonlib_error(&self, id: &Option<u32>, code: i32, error: &str) {
        log::warn!("on_error {:?} {} {}", id, code, error);
    }
}
