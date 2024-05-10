use tokio_test::assert_ok;
use tonlib::meta::{IpfsLoader, IpfsLoaderConfig};

mod common;

#[tokio::test]
async fn test_ipfs_http_gateway() {
    common::init_logging();
    let config = IpfsLoaderConfig::http_gateway("https://cloudflare-ipfs.com/ipfs/");
    let loader = assert_ok!(IpfsLoader::new(&config));
    let result = assert_ok!(
        loader
            .load_utf8_lossy("bafkreiast4fqlkp4upyu2cvo7fn7aabjusx765yzvqitsr4rpwfvhjguhy")
            .await
    );
    log::info!("{}", result);
    assert!(result.contains("BOLT"));
}

/// Requires IPFS node running on localhost:5001.
///
/// Check `compose/README.md` for details
#[tokio::test]
#[ignore]
async fn test_ipfs_node() {
    common::init_logging();
    let config = IpfsLoaderConfig::ipfs_node("http://localhost:5001");
    let loader = assert_ok!(IpfsLoader::new(&config));
    let result = assert_ok!(assert_ok!(
        tokio::spawn(async move {
            loader
                .load_utf8_lossy("bafkreiast4fqlkp4upyu2cvo7fn7aabjusx765yzvqitsr4rpwfvhjguhy")
                .await
        })
        .await
    ));
    log::info!("{}", result);
    assert!(result.contains("BOLT"));
}
