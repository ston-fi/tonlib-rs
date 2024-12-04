use sha2::{Digest, Sha256};
use tokio_test::assert_ok;
use tonlib_client::contract::{NftCollectionContract, NftItemContract, TonContractFactory};
use tonlib_client::meta::{LoadMeta, MetaDataContent, NftColletionMetaLoader, NftItemMetaLoader};
use tonlib_core::TonHash;

mod common;

#[tokio::test]
async fn test_get_nft_data() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQBKwtMZSZurMxGp7FLZ_lM9t54_ECEsS46NLR3qfIwwTnKW".parse()
    ));
    assert_ok!(contract.get_nft_data().await);
    // let x= contract.get_nft_data().await;
    // log::info!("(!!!) NftItemData: {:#?}", x);
}

#[tokio::test]
async fn test_get_nft_collection_data() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract =
        factory.get_contract(&"EQB2iHQ9lmJ9zvYPauxN9hVOfHL3c_fuN5AyRq5Pm84UH6jC".parse()?);
    assert_ok!(contract.get_collection_data().await);
    Ok(())
}

#[tokio::test]
async fn test_get_nft_address_by_index() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract = factory.get_contract(&assert_ok!(
        "EQB2iHQ9lmJ9zvYPauxN9hVOfHL3c_fuN5AyRq5Pm84UH6jC".parse()
    ));
    assert_ok!(contract.get_nft_address_by_index(2).await);
    Ok(())
}

// ---------------------nft get item metadata tests

// #[tokio::test]
// async fn test_get_nft_content_uri_OLD() -> anyhow::Result<()> {
//     common::init_logging();
//     let client = common::new_mainnet_client().await;
//     let factory = TonContractFactory::builder(&client).build().await?;
//     let contract = factory.get_contract(&assert_ok!(
//         "EQCGZEZZcYO9DK877fJSIEpYMSvfui7zmTXGhq0yq1Ce1Mb6".parse()
//     ));
//     let res = assert_ok!(contract.get_nft_data().await);
//
//     // Предположительно делить здесь.
//     let x = MyStruct { x: 42 };
//     log::info!("{:#?}", res);
//     assert_eq!(
//         res.individual_content,
//         MetaDataContent::External {
//             uri: "https://nft.fragment.com/number/88805397120.json".to_string()
//         }
//     );
//     let meta_loader = assert_ok!(NftItemMetaLoader::default());
//     let content_res = assert_ok!(meta_loader.load(&res.individual_content).await);
//     assert_eq!(
//         content_res.name.as_ref().unwrap(),
//         &String::from("+888 0539 7120")
//     );
//     assert_eq!(
//         content_res.image.as_ref().unwrap(),
//         &String::from("https://nft.fragment.com/number/88805397120.webp")
//     );
//     Ok(())
// }

#[tokio::test]
async fn test_get_nft_content_uri() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract = factory.get_contract(&assert_ok!(
        "EQCGZEZZcYO9DK877fJSIEpYMSvfui7zmTXGhq0yq1Ce1Mb6".parse()
    ));
    let res = assert_ok!(contract.get_nft_data().await);

    let expected_uri = "https://nft.fragment.com/number/88805397120.json".to_string();
    assert_eq!(
        res.individual_content,
        MetaDataContent::External { uri: expected_uri }
    );
    Ok(())
}

#[tokio::test]
async fn test_get_load_content_by_uri() -> anyhow::Result<()> {
    common::init_logging();
    let meta_loader = assert_ok!(NftItemMetaLoader::default());

    let request_uri = "https://nft.fragment.com/number/88805397120.json".to_string();
    let md_content = MetaDataContent::External { uri: request_uri };
    let md_content_res = assert_ok!(meta_loader.load(&md_content).await);

    let expected_phone = "+888 0539 7120".to_string();
    let expected_webp = "https://nft.fragment.com/number/88805397120.webp".to_string();

    assert_eq!(md_content_res.name.as_ref().unwrap(), &expected_phone);
    assert_eq!(md_content_res.image.as_ref().unwrap(),&expected_webp);

    Ok(())
}

#[tokio::test]
async fn test_get_nft_content_arkenston() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQDhR36C8pSVtyhOFtE9nh2DFq4WYUbTZFmvjfnShlrXq2cz".parse()
    ));
    let res = assert_ok!(contract.get_nft_data().await);

    // Пилить ниже
    let meta_loader = assert_ok!(NftItemMetaLoader::default());
    let content_res = assert_ok!(meta_loader.load(&res.individual_content).await);
    assert_eq!(
        content_res.image.unwrap(),
        "https://static.ston.fi/stake-nft/i4.jpg"
    );
    assert_eq!(content_res.name.unwrap(), "ARKENSTON NFT");
    Ok(())
}

#[tokio::test]
async fn test_get_nft_content_some() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract = factory.get_contract(&assert_ok!(
        "EQCiXgoveScGKGGqo50HbmwP3goKJaEfu9QmeBRJ-jbRxM21".parse()
    ));
    let res = assert_ok!(contract.get_nft_data().await);

    // Пилить здесь.
    let meta_loader = assert_ok!(NftItemMetaLoader::default());
    let content_res = assert_ok!(meta_loader.load(&res.individual_content).await);
    assert_eq!(
        content_res.image.unwrap(),
        "https://mars.tonplanets.com/i/biomes/4v4.jpg"
    );
    assert_eq!(content_res.name.unwrap(), "Anda");
    Ok(())
}

// ---------------------nft get collection metadata tests

#[tokio::test]
async fn test_get_nft_collection_content_uri() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_archive_mainnet_client().await;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract = factory.get_contract(&assert_ok!(
        "EQAOQdwdw8kGftJCSFgOErM1mBjYPe4DBPq8-AhF6vr9si5N".parse()
    ));
    let res = assert_ok!(contract.get_collection_data().await);

    assert_eq!(
        res.collection_content,
        MetaDataContent::External {
            uri: "https://nft.fragment.com/numbers.json".to_string()
        }
    );

    let meta_loader = NftColletionMetaLoader::default()?;
    let content_res = assert_ok!(
        meta_loader.load(&res.collection_content).await
    );
    assert_eq!(
        content_res.name.as_ref().unwrap(),
        &String::from("Anonymous Telegram Numbers")
    );
    assert_eq!(
        content_res.image.as_ref().unwrap(),
        &String::from("https://nft.fragment.com/numbers.svg")
    );
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

#[tokio::test]
async fn test_my_first_test() -> anyhow::Result<()> {
    println!("Hello, world!");
    Ok(())
}