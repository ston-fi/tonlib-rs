use tonlib::contract::{NftCollectionContract, NftItemContract, TonContract};
use tonlib::meta::*;

mod common;

#[tokio::test]
async fn test_get_nft_data() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let contract = TonContract::new(
        &client,
        &"EQBKwtMZSZurMxGp7FLZ_lM9t54_ECEsS46NLR3qfIwwTnKW".parse()?,
    );
    contract.get_nft_data().await?;
    Ok(())
}

#[tokio::test]
async fn test_get_nft_collection_data() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let contract = TonContract::new(
        &client,
        &"EQB2iHQ9lmJ9zvYPauxN9hVOfHL3c_fuN5AyRq5Pm84UH6jC".parse()?,
    );
    contract.get_collection_data().await?;
    Ok(())
}

#[tokio::test]
async fn test_get_nft_address_by_index() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let contract = TonContract::new(
        &client,
        &"EQB2iHQ9lmJ9zvYPauxN9hVOfHL3c_fuN5AyRq5Pm84UH6jC".parse()?,
    );
    contract.get_nft_address_by_index(2).await?;
    Ok(())
}

// ---------------------nft get item metadata tests

#[tokio::test]
async fn test_get_nft_content_uri() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let contract = TonContract::new(
        &client,
        &"EQCGZEZZcYO9DK877fJSIEpYMSvfui7zmTXGhq0yq1Ce1Mb6".parse()?,
    );
    let res = contract.get_nft_data().await?;
    assert_eq!(
        res.individual_content,
        MetaDataContent::External {
            uri: "https://nft.fragment.com/number/88805397120.json".to_string()
        }
    );
    let meta_loader = NftItemMetaLoader::default()?;
    let content_res = meta_loader.load(&res.individual_content).await?;
    assert_eq!(
        content_res.name.as_ref().unwrap(),
        &String::from("+888 0539 7120")
    );
    assert_eq!(
        content_res.image.as_ref().unwrap(),
        &String::from("https://nft.fragment.com/number/88805397120.webp")
    );
    Ok(())
}

#[tokio::test]
async fn test_get_nft_content_arkenston() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_test_client().await?;
    let contract = TonContract::new(
        &client,
        &"EQDhR36C8pSVtyhOFtE9nh2DFq4WYUbTZFmvjfnShlrXq2cz".parse()?,
    );
    let res = contract.get_nft_data().await?;
    let meta_loader = NftItemMetaLoader::default()?;
    let content_res = meta_loader.load(&res.individual_content).await?;
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
    let client = common::new_test_client().await?;
    let contract = TonContract::new(
        &client,
        &"EQCiXgoveScGKGGqo50HbmwP3goKJaEfu9QmeBRJ-jbRxM21".parse()?,
    );
    let res = contract.get_nft_data().await?;
    let meta_loader = NftItemMetaLoader::default()?;
    let content_res = meta_loader.load(&res.individual_content).await?;
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
    let client = common::new_test_client().await?;
    let contract = TonContract::new(
        &client,
        &"EQAOQdwdw8kGftJCSFgOErM1mBjYPe4DBPq8-AhF6vr9si5N".parse()?,
    );
    let res = contract.get_collection_data().await?;

    assert_eq!(
        res.collection_content,
        MetaDataContent::External {
            uri: "https://nft.fragment.com/numbers.json".to_string()
        }
    );

    let meta_loader = NftColletionMetaLoader::default()?;
    let content_res = meta_loader.load(&res.collection_content).await?;
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
    let client = common::new_test_client().await?;
    let contract = TonContract::new(
        &client,
        &"EQCshJXbbcn7cvSkaM0Z8NyI-2pNCJC5RTGZB-cRF-Pax1lY".parse()?,
    );
    let res = contract.get_collection_data().await?;
    let meta_loader = NftColletionMetaLoader::default()?;
    let content_res = meta_loader.load(&res.collection_content).await?;
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
    let client = common::new_test_client().await?;
    let contract = TonContract::new(
        &client,
        &"EQCbOjwru5tBb2aaXZEHbiTCVIYQ6yDNAe8SSZkP4CozibHM".parse()?,
    );
    let res = contract.get_nft_data().await?;
    let meta_loader = NftColletionMetaLoader::default()?;
    let content_res = meta_loader.load(&res.individual_content).await?;
    assert_eq!(content_res.name.unwrap(), "Pokemon Pikachu #013 💎");
    assert_eq!(
        content_res.image.unwrap(),
        "https://s.getgems.io/nft/c/64284ddbde940b5d6ebc34f8/12/image.png"
    );

    Ok(())
}
