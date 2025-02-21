use tonlib_client::contract::{BlockchainLibraryProvider, LibraryProvider};
use tonlib_core::cell::CellBuilder;
use tonlib_core::TonHash;

mod common;

#[tokio::test]
async fn test_load_libraries() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;

    let lib_hash1 = TonHash::from([
        159, 49, 244, 244, 19, 163, 172, 203, 112, 108, 136, 150, 42, 198, 157, 89, 16, 59, 1, 58,
        10, 221, 207, 174, 237, 93, 215, 60, 24, 250, 152, 168,
    ]);
    log::debug!("known_lib: {lib_hash1:?}");
    let lib_hash2 =
        TonHash::from_hex("f05e730bac652b0414b4673644999c81b8bd28595804c014fdf8078282799729")?;
    log::debug!("unknown_lib: {lib_hash2:?}");

    let library_loader = BlockchainLibraryProvider::new(&client, None);
    let loaded_libs = library_loader
        .load_libraries(&[lib_hash1.clone(), lib_hash1.clone(), lib_hash2.clone()])
        .await?;

    assert_eq!(loaded_libs.len(), 2);
    assert_eq!(loaded_libs[0].cell_hash(), lib_hash1);
    assert_eq!(loaded_libs[1].cell_hash(), lib_hash2);
    Ok(())
}

#[tokio::test]
async fn test_get_no_lib() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;

    let library_loader = BlockchainLibraryProvider::new(&client, None);
    let libs = library_loader
        .get_libs(&[CellBuilder::new().build()?.to_arc()], None)
        .await?;
    assert_eq!(libs.keys, Vec::new());
    assert_eq!(libs.dict_boc.len(), 0);
    Ok(())
}
