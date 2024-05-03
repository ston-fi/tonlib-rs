use futures::future::try_join_all;
use rand::Rng;
use tokio_test::assert_ok;
use tonlib::address::TonAddress;
use tonlib::client::TonClient;
use tonlib::contract::{JettonMasterContract, JettonWalletContract, TonContractFactory};

mod common;

const NUM_RUNNERS: usize = 1024;
const NUM_ITERATIONS: usize = 1000000;

#[ignore]
#[tokio::test]
async fn load_test_smc_methods() {
    common::init_logging();
    let client = common::new_testnet_client().await;
    let mut handles = vec![];
    for _ in 0..NUM_RUNNERS {
        let c = client.clone();

        let h = tokio::spawn(async move { smc_methods_runner(c).await });
        handles.push(h);
    }
    let r = assert_ok!(try_join_all(handles).await);
    log::info!("Result: {:?}", r);
}

const NUM_WALLETS: usize = 16;
const WALLETS: [&str; NUM_WALLETS] = [
    "EQDxi3M--3eDI4PD3W5WVJ8POqpJqWXP6YbYw5r9xZ91vyg9",
    "EQDxipX3kUDcNhNp63216SWqGoe83OSla6g2-DHU1ICi-uaC",
    "EQDxlPltD8KkB8vexZN1t4tfB-7lSnEZ8NxfmIPryWGBA2yQ",
    "EQDxmIFMZyODDP8mr41NXnfxfR1ip5wrTR12kv1lx5jBMHxO",
    "EQDxnn5aUMNvo-ZEs5L1LfWX4h-gPnYC2CacYdlXgvdtq3Bp",
    "EQDxnsi_XGO2M2B-N6MzNPi65Fo_zCQE9XcX_PMHKXAp7NvH",
    "EQDxoO4mUCB7fvJpCxlf0GaWNwYlsyi3CmpjbqFG2FPqHIFJ",
    "EQDxokDmX3OwJxHLTs_7T2l8bmfN7t8Wvgzc6anWutCiqZ_q",
    "EQDxpvMrnC1nFkGgq-8Eb7eDDKHRC_QpDuwWfC88jFaCQpLr",
    "EQDxqxT1zemcb4vfO1azdErbfomWJc6ZvhCMQKshiQntSifa",
    "EQDxrjcK6KJKxwxdDAOWQr0cK87br1hXcJMMHmd7C8XxlSuA",
    "EQDxsbQB2vAOgGEl3K6mJSojg1w87QsT0IA8zmSJ3jMEk12I",
    "EQDxsdI-vLAd61XBJKf_bORc0EfEwF5H7HpaJ5rG3smnGxaY",
    "EQDxutETPgCT9vnmHwus7VVLb1tS487RCVQtXgV70-6SR2Ax",
    "EQDxyJiWpQXWMIrB3db9HF0Mc87BzWSA1mPNYkljoBkVo6Lp",
    "EQDxzfv9TiXQYFHAnz5r-2vyt-h50Sc1WkUn9jpZ4s3K5xJX",
];

const NUM_JETTONS: usize = 4;
const JETTONS: [&str; NUM_JETTONS] = [
    "EQBynBO23ywHy_CgarY9NK9FTz0yDsG82PtcbSTQgGoXwiuA",
    "EQB-MPwrd1G6WKNkLz_VnV6WqBDd142KMQv-g1O-8QUA3728",
    "EQDo_ZJyQ_YqBzBwbVpMmhbhIddKtRP99HugZJ14aFscxi7B",
    "EQDcBkGHmC4pTf34x3Gm05XvepO5w60DNxZ-XT4I6-UGG5L5",
];

async fn smc_methods_runner(client: TonClient) {
    #[cfg(not(feature = "state_cache"))]
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    #[cfg(feature = "state_cache")]
    let factory = assert_ok!(
        TonContractFactory::builder(&client)
            .with_default_cache()
            .build()
            .await
    );
    for _ in 0..NUM_ITERATIONS {
        let wallet_index = get_random(NUM_WALLETS);
        let jetton_index = get_random(NUM_JETTONS);
        let wallet_addr: TonAddress = assert_ok!(WALLETS[wallet_index].parse());
        let jetton_addr: TonAddress = assert_ok!(JETTONS[jetton_index].parse());
        let jetton = factory.get_contract(&jetton_addr);
        let jetton_wallet_addr = assert_ok!(jetton.get_wallet_address(&wallet_addr).await);
        let jetton_wallet = factory.get_contract(&jetton_wallet_addr);
        let wallet_data_result = jetton_wallet.get_wallet_data().await;
        match wallet_data_result {
            Ok(wallet_data) => {
                assert_eq!(wallet_data.owner_address, wallet_addr);
                assert_eq!(wallet_data.master_address, jetton_addr);
            }
            Err(err) => {
                log::info!("Error occured: {}", err);
            }
        }
    }
}

fn get_random(max: usize) -> usize {
    let mut rng = rand::thread_rng();
    rng.gen_range(0..max)
}
