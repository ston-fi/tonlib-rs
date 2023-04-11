use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tonlib::client::{TonConnection, TonFunctions, DEFAULT_CONNECTION_PARAMS};
use tonlib::tl::types::{KeyStoreType, SyncState, UpdateSyncState};
use tonlib::tl::TonNotification;

use crate::common::TEST_TON_CONNECTION_CALLBACK;

mod common;

#[tokio::test]
async fn connection_init_works() -> anyhow::Result<()> {
    common::init_logging();
    let conn = TonConnection::new(TEST_TON_CONNECTION_CALLBACK.clone())?;
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

#[tokio::test]
async fn connection_sync_works() -> anyhow::Result<()> {
    common::init_logging();
    let conn = TonConnection::connect(
        &DEFAULT_CONNECTION_PARAMS,
        TEST_TON_CONNECTION_CALLBACK.clone(),
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
        .get_account_state("EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR")
        .await;
    println!("{:?}", r);
    assert!(r.is_ok());
    assert!(flag.load(Ordering::Acquire));
    Ok(())
}
