use tonlib::{
    contract::TonContract,
    nft::{NftCollectionContract, NftItemContract},
};

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
