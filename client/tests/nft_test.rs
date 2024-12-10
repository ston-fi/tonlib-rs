use num_bigint::BigUint;
use sha2::{Digest, Sha256};
use std::str::FromStr;
use tokio_test::assert_ok;
use tonlib_client::contract::{
    NftCollectionContract, NftCollectionData, NftItemContract, NftItemData, TonContractFactory,
};
use tonlib_client::meta::MetaDataContent::External;
use tonlib_client::meta::{LoadMeta, MetaDataContent, NftColletionMetaLoader, NftItemMetaLoader};
use tonlib_core::{TonAddress, TonHash};

mod common;

// ---- Tests methods is work only.
#[tokio::test]
async fn test_get_nft_data() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQBKwtMZSZurMxGp7FLZ_lM9t54_ECEsS46NLR3qfIwwTnKW".parse()
    ));
    assert_ok!(contract.get_nft_data().await);
}

#[tokio::test]
async fn test_get_nft_collection_data() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract = factory.get_contract(
        &"EQB2iHQ9lmJ9zvYPauxN9hVOfHL3c_fuN5AyRq5Pm84UH6jC".parse()?
    );
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

// ---- Tests methods return valid data.
#[tokio::test]
async fn test_get_nft_data_is_valid() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract = factory.get_contract(&assert_ok!(
        "EQCGZEZZcYO9DK877fJSIEpYMSvfui7zmTXGhq0yq1Ce1Mb6".parse()
    ));
    let res = assert_ok!(contract.get_nft_data().await);

    let expected_collection_address = assert_ok!(TonAddress::from_base64_url(
        &"EQAOQdwdw8kGftJCSFgOErM1mBjYPe4DBPq8-AhF6vr9si5N".to_string()
    ));
    let expected_owner_address = assert_ok!(TonAddress::from_base64_url(
        &"EQCgjHh831e9_qlCWLgaAwEIQ8qOolUT831vJF0bau6LMV5G".to_string()
    ));
    let expected_index = assert_ok!(BigUint::from_str(
        "15995005474673311991943775795727481451058346239240361725119718297821926435889",
    ));
    let expected_response = NftItemData {
        init: true,
        index: expected_index,
        collection_address: expected_collection_address,
        owner_address: expected_owner_address,
        individual_content: External {
            uri: "https://nft.fragment.com/number/88805397120.json".to_string(),
        },
    };
    assert_eq!(res, expected_response);
    Ok(())
}

#[tokio::test]
async fn test_get_nft_collection_data_is_valid() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_archive_mainnet_client().await;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract = factory.get_contract(&assert_ok!(
        "EQAOQdwdw8kGftJCSFgOErM1mBjYPe4DBPq8-AhF6vr9si5N".parse()
    ));
    let res = assert_ok!(contract.get_collection_data().await);

    let expected_owner_address = assert_ok!(TonAddress::from_base64_url(
        &"EQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM9c".to_string()
    ));
    let expected_response = NftCollectionData {
        next_item_index: -1,
        owner_address: expected_owner_address,
        collection_content: External {
            uri: "https://nft.fragment.com/numbers.json".to_string(),
        },
    };
    assert_eq!(res, expected_response);
    Ok(())
}

#[tokio::test]
async fn test_get_nft_address_by_index_is_valid() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = TonContractFactory::builder(&client).build().await?;
    let contract = factory.get_contract(&assert_ok!(
        "EQB2iHQ9lmJ9zvYPauxN9hVOfHL3c_fuN5AyRq5Pm84UH6jC".parse()
    ));

    let res_0 = assert_ok!(contract.get_nft_address_by_index(0).await);
    let res_2 = assert_ok!(contract.get_nft_address_by_index(2).await);
    let res_1 = assert_ok!(contract.get_nft_address_by_index(1).await);

    let expected_addr_0 = assert_ok!(TonAddress::from_base64_url(
        &"EQBKwtMZSZurMxGp7FLZ_lM9t54_ECEsS46NLR3qfIwwTnKW".to_string()
    ));
    let expected_addr_1 = assert_ok!(TonAddress::from_base64_url(
        &"EQB6rnPIZr8dXmLy0xVp4lTe1AlYRwOUghEG9zzCcCkCp8IS".to_string()
    ));
    let expected_addr_2 = assert_ok!(TonAddress::from_base64_url(
        &"EQD0VQNu41wZmWMQjXfifnljGR0vOAULh0stBLItskMavwH0".to_string()
    ));
    assert_eq!(res_0, expected_addr_0);
    assert_eq!(res_1, expected_addr_1);
    assert_eq!(res_2, expected_addr_2);
    Ok(())
}
