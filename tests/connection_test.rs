use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tonlib::address::TonAddress;
use tonlib::client::{
    MultiConnectionCallback, TonClientError, TonClientInterface, TonConnection,
    TonConnectionCallback, DEFAULT_CONNECTION_PARAMS, LOGGING_CONNECTION_CALLBACK,
    NOOP_CONNECTION_CALLBACK,
};
use tonlib::tl::{
    KeyStoreType, SyncState, TonFunction, TonNotification, TonResult, UpdateSyncState,
};

mod common;

#[tokio::test]
async fn test_connection_init() -> anyhow::Result<()> {
    common::init_logging();
    let conn = TonConnection::new(LOGGING_CONNECTION_CALLBACK.clone())?;
    let r = conn
        .init(
            tonlib::config::MAINNET_CONFIG,
            None,
            false,
            false,
            KeyStoreType::InMemory,
        )
        .await;
    println!("{:?}", r);
    assert!(r.is_ok());
    let lvl = conn.get_log_verbosity_level().await?;
    println!("Log verbosity level: {}", lvl);
    Ok(())
}

struct TestConnectionCallback {
    pub num_invoke: AtomicU32,
    pub num_invoke_result: AtomicU32,
    pub num_result_parse_error: AtomicU32,
}

impl TestConnectionCallback {
    pub fn new() -> TestConnectionCallback {
        TestConnectionCallback {
            num_invoke: Default::default(),
            num_invoke_result: Default::default(),
            num_result_parse_error: Default::default(),
        }
    }
}

#[allow(unused_variables)]
impl TonConnectionCallback for TestConnectionCallback {
    fn on_invoke(&self, tag: &str, request_id: u32, function: &TonFunction) {
        self.num_invoke.fetch_add(1, Ordering::SeqCst);
    }

    fn on_invoke_result(
        &self,
        tag: &str,
        request_id: u32,
        method: &str,
        duration: &Duration,
        result: &Result<TonResult, TonClientError>,
    ) {
        self.num_invoke_result.fetch_add(1, Ordering::SeqCst);
    }

    fn on_ton_result_parse_error(
        &self,
        tag: &str,
        request_extra: Option<&str>,
        result: &TonResult,
    ) {
        self.num_result_parse_error.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn test_connection_callback() -> anyhow::Result<()> {
    common::init_logging();
    let test_callback = Arc::new(TestConnectionCallback::new());
    let multi_callback = Arc::new(MultiConnectionCallback::new(vec![
        NOOP_CONNECTION_CALLBACK.clone(),
        test_callback.clone(),
        LOGGING_CONNECTION_CALLBACK.clone(),
    ]));
    let conn = TonConnection::new(multi_callback)?;
    let r = conn
        .init(
            tonlib::config::MAINNET_CONFIG,
            None,
            false,
            false,
            KeyStoreType::InMemory,
        )
        .await;
    println!("{:?}", r);
    assert!(r.is_ok());
    let lvl = conn.get_log_verbosity_level().await?;
    println!("Log verbosity level: {}", lvl);
    assert_eq!(2, test_callback.num_invoke.load(Ordering::SeqCst));
    assert_eq!(2, test_callback.num_invoke_result.load(Ordering::SeqCst));
    assert_eq!(
        0,
        test_callback.num_result_parse_error.load(Ordering::SeqCst)
    );
    Ok(())
}

#[tokio::test]
async fn test_connection_sync() -> anyhow::Result<()> {
    common::init_logging();
    let conn = TonConnection::connect(
        &DEFAULT_CONNECTION_PARAMS,
        LOGGING_CONNECTION_CALLBACK.clone(),
    )
    .await?;
    let mut receiver = conn.subscribe();
    let flag = Arc::new(AtomicBool::new(false));
    let flag_copy = flag.clone();
    tokio::spawn(async move {
        let synced = TonNotification::UpdateSyncState(UpdateSyncState {
            sync_state: SyncState::Done,
        });
        while let Ok(n) = receiver.recv().await {
            if *n == synced {
                println!("Synchronized");
                flag_copy.store(true, Ordering::Release);
            }
        }
    });
    let r = conn
        .get_account_state(&TonAddress::from_base64_url(
            "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR",
        )?)
        .await;
    println!("{:?}", r);
    assert!(r.is_ok());
    assert!(flag.load(Ordering::Acquire));
    Ok(())
}
