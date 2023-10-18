use tonlib::meta::{IpfsLoader, IpfsLoaderConfig};

mod common;

#[tokio::test]
async fn test_ipfs_http_gateway() -> anyhow::Result<()> {
    common::init_logging();
    let config = IpfsLoaderConfig::http_gateway("https://cloudflare-ipfs.com/ipfs/");
    let loader = IpfsLoader::new(&config)?;
    let result = loader
        .load_utf8_lossy("bafkreiast4fqlkp4upyu2cvo7fn7aabjusx765yzvqitsr4rpwfvhjguhy")
        .await?;
    println!("{}", result);
    assert!(result.contains("BOLT"));
    Ok(())
}

/// Requires IPFS node running on localhost:5001.
///
/// Check `compose/README.md` for details
#[tokio::test]
#[ignore]
async fn test_ipfs_node() -> anyhow::Result<()> {
    common::init_logging();
    let config = IpfsLoaderConfig::ipfs_node("http://localhost:5001");
    let loader = IpfsLoader::new(&config)?;
    let result = tokio::spawn(async move {
        let r = loader
            .load_utf8_lossy("bafkreiast4fqlkp4upyu2cvo7fn7aabjusx765yzvqitsr4rpwfvhjguhy")
            .await;
        r
    })
    .await??;
    println!("{}", result);
    assert!(result.contains("BOLT"));
    Ok(())
}
