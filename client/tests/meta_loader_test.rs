use serde_json::json;
use tokio_test::assert_ok;
use tonlib_client::meta::{
    LoadMeta, MetaDataContent, NftCollectionMetaData, NftColletionMetaLoader, NftItemMetaData,
    NftItemMetaLoader,
};

mod common;

// ---- Nft item metadata load tests
#[tokio::test]
async fn test_load_item_metadata() -> anyhow::Result<()> {
    common::init_logging();
    let meta_loader = assert_ok!(NftItemMetaLoader::default());
    let content = MetaDataContent::External {
        uri: "https://nft.fragment.com/number/88805397120.json".to_string(),
    };

    let expected_res = NftItemMetaData {
        name: Some("+888 0539 7120".to_string()),
        description: Some(
            "The anonymous number +888 0539 7120 that can be used to create a Telegram \
             account that is not tied to a SIM card."
                .to_string(),
        ),
        image: Some("https://nft.fragment.com/number/88805397120.webp".to_string()),
        content_url: None,
        attributes: None,
    };

    let res = assert_ok!(meta_loader.load(&content).await);
    assert_eq!(expected_res, res);
    Ok(())
}

#[tokio::test]
async fn test_load_item_metadata_arkenston() -> anyhow::Result<()> {
    common::init_logging();
    let meta_loader = assert_ok!(NftItemMetaLoader::default());
    let content = MetaDataContent::External {
        uri: "https://meta.ston.fi/meta/stake/v1/0:E1477E82F29495B7284E16D13D9E1D8316AE166146D36459AF8DF9D2865AD7AB.json".to_string(),
    };

    let expected_res = NftItemMetaData {
        name: Some("ARKENSTON NFT".to_string()),
        description: Some("Staked 30.000000000 STON on STON.fi from 04 Sep 2023.".to_string()),
        image: Some("https://static.ston.fi/stake-nft/i4.jpg".to_string()),
        content_url: None,
        attributes: None,
    };

    let res = assert_ok!(meta_loader.load(&content).await);
    assert_eq!(expected_res, res);
    Ok(())
}

#[tokio::test]
async fn test_load_item_metadata_with_attributes() -> anyhow::Result<()> {
    common::init_logging();
    let meta_loader = assert_ok!(NftItemMetaLoader::default());
    let content = MetaDataContent::External {
        uri: "https://s.getgems.io/nft/c/64284ddbde940b5d6ebc34f8/12/meta.json".to_string(),
    };

    let attributes = json!([
        {"trait_type": "Pikachu","value": "100"},
        {"trait_type": "Pokemon","value": "100"},
        {"trait_type": "cocktail","value": "100"},
        {"trait_type": "rocks","value": "100"}
    ]);
    let expected_res = NftItemMetaData {
        name: Some("Pokemon Pikachu #013 💎".to_string()),
        description: Some(
            "The legendary Pokemon Pikachu from the exclusive collection. Gather everyone!"
                .to_string(),
        ),
        image: Some("https://s.getgems.io/nft/c/64284ddbde940b5d6ebc34f8/12/image.png".to_string()),
        attributes: Some(attributes),
        content_url: None,
    };

    let res = assert_ok!(meta_loader.load(&content).await);
    assert_eq!(expected_res, res);
    Ok(())
}

// ---- Nft collection metadata load tests
#[tokio::test]
async fn test_load_collection_metadata_content_arkenston() -> anyhow::Result<()> {
    common::init_logging();
    let meta_loader = assert_ok!(NftColletionMetaLoader::default());
    let content = MetaDataContent::External {
        uri: "https://meta.ston.fi/meta/stake-collection/v1/0:AC8495DB6DC9FB72F4A468CD19F0DC88FB6A4D0890B945319907E71117E3DAC7.json".to_string()
    };

    let social_links = json!(["https://app.ston.fi/staking"]);
    let expected_res = NftCollectionMetaData {
        image: Some("https://static.ston.fi/stake-nft/i1.jpg".to_string()),
        name: Some("ARKENSTON NFT".to_string()),
        description: Some("psSTON STON.fi Stake".to_string()),
        social_links: Some(social_links),
        marketplace: None,
    };

    let res = assert_ok!(meta_loader.load(&content).await);
    assert_eq!(expected_res, res);
    Ok(())
}

// Note: HTTP-Response to this URI has an additional key "external_link".
// We don't handle it. It's okay. FYI.
#[tokio::test]
async fn test_load_collection_metadata_content() -> anyhow::Result<()> {
    common::init_logging();
    let meta_loader = NftColletionMetaLoader::default()?;
    let content = MetaDataContent::External {
        uri: "https://nft.fragment.com/numbers.json".to_string(),
    };

    let expected_res = NftCollectionMetaData {
        image: Some("https://nft.fragment.com/numbers.svg".to_string()),
        name: Some("Anonymous Telegram Numbers".to_string()),
        description: Some(
            "These anonymous numbers can be used to create Telegram accounts \
            that are not tied to SIM cards."
                .to_string(),
        ),
        social_links: None,
        marketplace: None,
    };

    let res = assert_ok!(meta_loader.load(&content).await);
    assert_eq!(expected_res, res);
    Ok(())
}
