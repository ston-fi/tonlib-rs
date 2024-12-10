use sha2::{Digest, Sha256};
use tokio_test::assert_ok;
use tonlib_client::contract::{NftCollectionContract, NftItemContract, TonContractFactory};
use tonlib_client::meta::{LoadMeta, MetaDataContent, NftCollectionMetaData, NftColletionMetaLoader, NftItemMetaData, NftItemMetaLoader};
use tonlib_core::TonHash;

mod common;

#[tokio::test]
async fn test_load_item_meta_data_by_uri() -> anyhow::Result<()> {
    common::init_logging();
    let meta_loader = assert_ok!(NftItemMetaLoader::default());
    let md_content = MetaDataContent::External {
        uri: "https://nft.fragment.com/number/88805397120.json".to_string(),
    };

    let expected_response = NftItemMetaData {
        name: Some("+888 0539 7120".to_string()),
        description: Some(
            "The anonymous number +888 0539 7120 that can be used to create a Telegram \
             account that is not tied to a SIM card.".to_string(),
        ),
        image: Some("https://nft.fragment.com/number/88805397120.webp".to_string()),
        content_url: None,
        attributes: None,
    };

    let response = assert_ok!(meta_loader.load(&md_content).await);
    assert_eq!(expected_response, response);
    Ok(())
}

#[tokio::test]
async fn test_load_item_meta_data_by_uri_arkenstone() -> anyhow::Result<()> {
    common::init_logging();
    let meta_loader = assert_ok!(NftItemMetaLoader::default());
    let md_content = MetaDataContent::External {
        uri: "https://meta.ston.fi/meta/stake/v1/0:E1477E82F29495B7284E16D13D9E1D8316AE166146D36459AF8DF9D2865AD7AB.json".to_string(),
    };

    let expected_response = NftItemMetaData {
        name: Some("ARKENSTON NFT".to_string()),
        description: Some("Staked 30.000000000 STON on STON.fi from 04 Sep 2023.".to_string()),
        image: Some("https://static.ston.fi/stake-nft/i4.jpg".to_string()),
        content_url: None,
        attributes: None,
    };

    let response = assert_ok!(meta_loader.load(&md_content).await);
    assert_eq!(expected_response, response);
    Ok(())
}

// -------------------- !!!
// ---------------------nft get collection metadata tests

#[tokio::test]
async fn test_get_nft_collection_content_uri() -> anyhow::Result<()> {
    common::init_logging();
    let meta_loader = NftColletionMetaLoader::default()?;
    let md_content = MetaDataContent::External {
        uri: "https://nft.fragment.com/numbers.json".to_string()
    };

    let expected_response = NftCollectionMetaData {
        image: Some("https://nft.fragment.com/numbers.svg".to_string()),
        name: Some("Anonymous Telegram Numbers".to_string()),
        description: Some(
            "These anonymous numbers can be used to create Telegram accounts \
            hat are not tied to SIM cards.".to_string()),
        social_links: None,
        marketplace: None,
    };

    let response = assert_ok!(meta_loader.load(&md_content).await);
    log::info!("{:?}", response);
    Ok(())
}

#[tokio::test]
async fn test_get_nft_collection_content_arkenston() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQCshJXbbcn7cvSkaM0Z8NyI-2pNCJC5RTGZB-cRF-Pax1lY".parse()
    ));
    let res = assert_ok!(contract.get_collection_data().await);
    let meta_loader = assert_ok!(NftColletionMetaLoader::default());
    let content_res = assert_ok!(meta_loader.load(&res.collection_content).await);
    assert_eq!(content_res.name.unwrap(), "ARKENSTON NFT");
    assert_eq!(
        content_res.image.unwrap(),
        "https://static.ston.fi/stake-nft/i1.jpg"
    );
    Ok(())
}

#[tokio::test]
async fn test_get_nft_collection_content_some() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQCbOjwru5tBb2aaXZEHbiTCVIYQ6yDNAe8SSZkP4CozibHM".parse()
    ));
    let res = assert_ok!(contract.get_nft_data().await);
    let meta_loader = assert_ok!(NftColletionMetaLoader::default());
    let content_res = assert_ok!(meta_loader.load(&res.individual_content).await);
    assert_eq!(content_res.name.unwrap(), "Pokemon Pikachu #013 💎");
    assert_eq!(
        content_res.image.unwrap(),
        "https://s.getgems.io/nft/c/64284ddbde940b5d6ebc34f8/12/image.png"
    );
    Ok(())
}

#[tokio::test]
async fn test_get_nft_content_external() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        factory.get_contract(&"EQDUF9cLVBH3BgziwOAIkezUdmfsDxxJHd6WSv0ChIUXYwCx".parse()?);
    let nft_data = contract.get_nft_data().await?;
    let internal = match nft_data.individual_content {
        MetaDataContent::Internal { dict } => dict,
        _ => panic!("Expected internal content"),
    };

    let expected_key = {
        let mut hasher: Sha256 = Sha256::new();
        hasher.update("public_keys");
        let slice = &hasher.finalize()[..];
        TryInto::<TonHash>::try_into(slice)?
    };
    assert!(internal.contains_key(&expected_key));
    Ok(())
}
