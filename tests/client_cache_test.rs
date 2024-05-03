#[cfg(feature = "state_cache")]
use std::time::Duration;

#[cfg(feature = "state_cache")]
use tokio::{self};
#[cfg(feature = "state_cache")]
use tokio_test::assert_ok;
#[cfg(feature = "state_cache")]
use tonlib::address::TonAddress;
#[cfg(feature = "state_cache")]
use tonlib::contract::TonContractFactory;
mod common;

#[tokio::test]
#[cfg(feature = "state_cache")]
async fn cache_get_raw_account_state_works() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(
        TonContractFactory::builder(&client)
            .with_default_cache()
            .build()
            .await
    );
    for _ in 0..100 {
        assert_ok!(
            factory
                .get_latest_account_state(assert_ok!(&TonAddress::from_base64_url(
                    "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR",
                )))
                .await
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

#[tokio::test]
#[cfg(feature = "state_cache")]
async fn cache_contract_state_works() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(
        TonContractFactory::builder(&client)
            .with_default_cache()
            .build()
            .await
    );
    for _ in 0..100 {
        assert_ok!(
            factory
                .get_latest_contract_state(assert_ok!(&TonAddress::from_base64_url(
                    "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR",
                )))
                .await
        );
    }
}
