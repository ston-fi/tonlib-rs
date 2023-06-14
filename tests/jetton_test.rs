use tonlib::address::TonAddress;
use tonlib::contract::TonContract;
use tonlib::jetton::{JettonContent, JettonContentLoader, JettonMasterContract};

mod common;

#[tokio::test]
async fn test_get_jetton_content_uri() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let address: TonAddress = "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR".parse()?;
    let contract = TonContract::new(&client, &address); //MOON jetton
    let res = contract.get_jetton_data().await?;
    assert_eq!(
        res.content,
        JettonContent::External {
            uri: "https://tarantini.dev/ston/moon.json".to_string()
        }
    );
    let loader = JettonContentLoader::default()?;
    let content_res = loader.load(&res.content).await?;
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("MOON"));
    assert_eq!(content_res.decimals.unwrap(), 0x9);

    Ok(())
}

#[tokio::test]
async fn test_get_jetton_content_internal_uri() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let address: TonAddress = "EQDCJL0iQHofcBBvFBHdVG233Ri2V4kCNFgfRT-gqAd3Oc86".parse()?;
    let contract = TonContract::new(&client, &address); //FunZee jetton
    let res = contract.get_jetton_data().await?;
    let loader = JettonContentLoader::default()?;
    let content_res = loader.load(&res.content).await?;
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("FNZ"));
    assert_eq!(content_res.decimals.unwrap(), 0x9);

    Ok(())
}

#[tokio::test]
async fn test_get_jetton_content_internal_uri_tgr() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let address: TonAddress = "EQAvDfWFG0oYX19jwNDNBBL1rKNT9XfaGP9HyTb5nb2Eml6y".parse()?;
    let contract = TonContract::new(&client, &address); //FunZee jetton
    let res = contract.get_jetton_data().await?;
    let loader = JettonContentLoader::default()?;
    let content_res = loader.load(&res.content).await?;
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("TGR"));
    assert_eq!(content_res.decimals, None);

    Ok(())
}

#[tokio::test]
async fn test_get_jetton_content_ipfs_uri() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let address: TonAddress = "EQD0vdSA_NedR9uvbgN9EikRX-suesDxGeFg69XQMavfLqIw".parse()?;
    let contract = TonContract::new(&client, &address); // BOLT jetton
    let res = contract.get_jetton_data().await?;
    let loader = JettonContentLoader::default()?;
    let content_res = loader.load(&res.content).await?;
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("BOLT"));
    println!("{:?}", content_res);
    println!("{:?}", content_res.image_data);
    assert_eq!(content_res.decimals.unwrap(), 0x9);

    Ok(())
}

#[tokio::test]
async fn test_get_semi_chain_layout_jetton_content() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let address: TonAddress = "EQB-MPwrd1G6WKNkLz_VnV6WqBDd142KMQv-g1O-8QUA3728".parse()?;
    let contract = TonContract::new(&client, &address); // jUSDC jetton
    let res = contract.get_jetton_data().await?;
    let loader = JettonContentLoader::default()?;
    let content_res = loader.load(&res.content).await?;
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("jUSDC"));
    assert_eq!(content_res.name.as_ref().unwrap(), &String::from("jUSDC"));
    assert_eq!(content_res.decimals.unwrap(), 0x6);

    Ok(())
}

#[tokio::test]
async fn test_get_wallet_address() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let address: TonAddress = "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR".parse()?;
    let contract = TonContract::new(&client, &address);
    let owner_address =
        TonAddress::from_base64_url("EQB2BtXDXaQuIcMYW7JEWhHmwHfPPwa-eoCdefiAxOhU3pQg")?;
    let wallet_address = contract.get_wallet_address(&owner_address).await?;
    assert_eq!(
        "EQCGY3OVLtD9KRcOsP2ldQDtuY0FMzV7wPoxjrFbayBXc23c",
        wallet_address.to_base64_std()
    );
    Ok(())
}
