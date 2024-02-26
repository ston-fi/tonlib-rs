#![cfg(feature = "interactive")]

use tonlib::address::TonAddress;
use tonlib::contract::{JettonMasterContract, TonContractFactory};
use tonlib::meta::*;

mod common;

#[tokio::test]
async fn test_get_jetton_content_uri() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await?;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        factory.get_contract(&"EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR".parse()?); // Moon jetton
    let res = contract.get_jetton_data().await?;
    assert_eq!(
        res.content,
        MetaDataContent::External {
            uri: "https://tarantini.dev/ston/moon.json".to_string()
        }
    );
    let meta_loader = JettonMetaLoader::default()?;
    let content_res = meta_loader.load(&res.content).await?;
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("MOON"));
    assert_eq!(content_res.decimals.unwrap(), 0x9);

    Ok(())
}

#[tokio::test]
async fn test_get_jetton_content_internal_uri() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await?;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        factory.get_contract(&"EQDCJL0iQHofcBBvFBHdVG233Ri2V4kCNFgfRT-gqAd3Oc86".parse()?); // Fanzee jetton
    let res = contract.get_jetton_data().await?;
    let meta_loader = JettonMetaLoader::default()?;
    let content_res = meta_loader.load(&res.content).await?;
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("FNZ"));
    assert_eq!(content_res.decimals.unwrap(), 0x9);

    Ok(())
}

#[tokio::test]
async fn test_get_jetton_content_internal_uri_jusdt() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await?;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        factory.get_contract(&"EQBynBO23ywHy_CgarY9NK9FTz0yDsG82PtcbSTQgGoXwiuA".parse()?); // jUSDT jetton
    let res = contract.get_jetton_data().await?;
    let meta_loader = JettonMetaLoader::default()?;
    let content_res = meta_loader.load(&res.content).await?;
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("jUSDT"));
    assert_eq!(content_res.decimals, Some(6));

    Ok(())
}

#[tokio::test]
async fn test_get_jetton_content_empty_external_meta() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await?;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        factory.get_contract(&"EQD-J6UqYQezuUm6SlPDnHwTxXqo4uHys_fle_zKvM5nYJkA".parse()?);
    let res = contract.get_jetton_data().await?;
    let meta_loader = JettonMetaLoader::default()?;
    let content_res = meta_loader.load(&res.content).await?;
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("BLKC"));
    assert_eq!(content_res.decimals, Some(8));

    Ok(())
}
#[tokio::test]
async fn test_get_jetton_content_ipfs_uri() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await?;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        factory.get_contract(&"EQD0vdSA_NedR9uvbgN9EikRX-suesDxGeFg69XQMavfLqIw".parse()?); // BOLT jetton
    let res = contract.get_jetton_data().await?;
    let meta_loader = JettonMetaLoader::default()?;
    let content_res = meta_loader.load(&res.content).await?;
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("BOLT"));
    println!("{:?}", content_res);
    println!("{:?}", content_res.image_data);
    assert_eq!(content_res.decimals.unwrap(), 0x9);

    Ok(())
}

#[tokio::test]
async fn test_get_semi_chain_layout_jetton_content() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await?;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        factory.get_contract(&"EQB-MPwrd1G6WKNkLz_VnV6WqBDd142KMQv-g1O-8QUA3728".parse()?); // jUSDC jetton
    let res = contract.get_jetton_data().await?;
    let meta_loader = JettonMetaLoader::default()?;
    let content_res = meta_loader.load(&res.content).await?;
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("jUSDC"));
    assert_eq!(
        content_res.name.as_ref().unwrap(),
        &String::from("USD Coin")
    );
    assert_eq!(content_res.decimals.unwrap(), 0x6);

    Ok(())
}

#[tokio::test]
async fn test_get_wallet_address() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await?;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        factory.get_contract(&"EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR".parse()?);
    let owner_address =
        TonAddress::from_base64_url("EQB2BtXDXaQuIcMYW7JEWhHmwHfPPwa-eoCdefiAxOhU3pQg")?;
    let wallet_address = contract.get_wallet_address(&owner_address).await?;
    assert_eq!(
        "EQCGY3OVLtD9KRcOsP2ldQDtuY0FMzV7wPoxjrFbayBXc23c",
        wallet_address.to_base64_std()
    );
    Ok(())
}

#[tokio::test]
async fn test_get_jetton_data_invalid_utf8_sequence() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await?;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        factory.get_contract(&"EQDX__KZ7A--poP3Newpo_zx4tQ-yl9yzRwlmg_vifxMEA8m".parse()?);
    let res = contract.get_jetton_data().await?;
    log::info!("DATA: {:?}", res);
    let meta_loader = JettonMetaLoader::default()?;
    let content_res = meta_loader.load(&res.content).await?;
    assert_eq!(
        content_res.symbol.as_ref().unwrap(),
        &String::from("DuRove's")
    );
    assert_eq!(content_res.decimals.unwrap(), 0x9);

    let contract =
        factory.get_contract(&"EQDoEAodkem9PJdk3W1mqjnkvRphNaWu0glIRzxQBNZuOIbP".parse()?);
    let res = contract.get_jetton_data().await?;
    log::info!("DATA: {:?}", res);
    let meta_loader = JettonMetaLoader::default()?;
    let content_res = meta_loader.load(&res.content).await?;
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("TFH"));
    assert_eq!(content_res.decimals.unwrap(), 0x9);

    Ok(())
}
