use lazy_static::lazy_static;
use std::sync::{Arc, Once};
use std::time::Duration;
use tokio::sync::broadcast::error::SendError;

use log::LevelFilter;
use log4rs::append::console::{ConsoleAppender, Target};
use log4rs::config::{Appender, Root};
use log4rs::Config;
use tonlib::{
    client::DefaultConnectionCallback,
    tl::{TonNotification, TonResult},
};
use tonlib::{
    client::{TonClient, TonClientError, TonConnectionCallback},
    tl::TlError,
};

static LOG: Once = Once::new();

lazy_static! {
    pub static ref TEST_TON_CONNECTION_CALLBACK: Arc<DefaultConnectionCallback> =
        Arc::new(DefaultConnectionCallback {});
}

pub fn init_logging() {
    LOG.call_once(|| {
        TonClient::set_log_verbosity_level(2);
        let stderr = ConsoleAppender::builder()
            .target(Target::Stderr)
            .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
                "{d(%Y-%m-%d %H:%M:%S%.6f)} {T} {h({l:>5.5} {t})} [{f}:{L}]- {m}{n}",
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

    fn on_tl_error(&self, tag: &String, error: &TlError) {
        log::warn!("[{}] Tl error: {}", tag, error);
    }

    fn on_invoke_result(
        &self,
        tag: &String,
        id: u32,
        method: &str,
        duration: &Duration,
        res: &Result<TonResult, TonClientError>,
    ) {
        log::trace!(
            "[{}] on_invoke_result:{:?} {} {} {:?}",
            tag,
            id,
            method,
            duration.as_micros(),
            res
        );
    }

    fn on_notification_ok(&self, tag: &String, notification: &TonNotification) {
        log::trace!("[{}] on_notification: {:?}", tag, notification);
    }

    fn on_notification_err(&self, tag: &String, e: SendError<Arc<TonNotification>>) {
        log::warn!("[{}] Error sending notification: {}", tag, e);
    }

    fn on_tonlib_error(&self, tag: &String, id: &Option<u32>, code: i32, error: &str) {
        log::warn!("[{}] on_error {:?} {} {}", tag, id, code, error);
    }

    fn on_invoke_result_send_error(
        &self,
        tag: &String,
        request_id: u32,
        method: &str,
        duration: &Duration,
        e: &Result<TonResult, TonClientError>,
    ) {
        log::warn!(
            "[{}] Error sending invoke result, method: {} request_id: {}, elapsed: {:?}: {:?}",
            tag,
            method,
            request_id,
            &duration,
            e
        );
    }

    fn on_ton_result_parse_error(&self, tag: &String, result: &TonResult) {
        log::warn!("[{}] Error parsing result: {}", tag, result);
    }
}
