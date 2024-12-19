use std::str::FromStr;

use tokio_test::assert_ok;
use tonlib_client::contract::{
    DefaultLibraryLoader, LibraryLoader, LibraryProvider, TonContractFactory,
};
use tonlib_core::cell::BagOfCells;
use tonlib_core::{TonAddress, TonHash};

mod common;

#[tokio::test]
async fn test_get_lib_by_hash() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;

    let expected_lib_id = TonHash::from([
        159, 49, 244, 244, 19, 163, 172, 203, 112, 108, 136, 150, 42, 198, 157, 89, 16, 59, 1, 58,
        10, 221, 207, 174, 237, 93, 215, 60, 24, 250, 152, 168,
    ]);
    log::info!("{:?}", expected_lib_id);

    let library_loader = DefaultLibraryLoader::new(&client);
    let maybe_lib = library_loader.get_library(&expected_lib_id).await?;
    assert!(maybe_lib.is_some());

    let lib = maybe_lib.unwrap();
    let lib_hash = lib.cell_hash();

    assert_eq!(expected_lib_id, lib_hash);

    Ok(())
}

#[tokio::test]
async fn test_get_libs_by_hash() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;

    let expected_lib_id = TonHash::from([
        159, 49, 244, 244, 19, 163, 172, 203, 112, 108, 136, 150, 42, 198, 157, 89, 16, 59, 1, 58,
        10, 221, 207, 174, 237, 93, 215, 60, 24, 250, 152, 168,
    ]);
    log::info!("{:?}", expected_lib_id);

    let library_loader = DefaultLibraryLoader::new(&client);
    let maybe_lib = library_loader
        .get_libraries(&[expected_lib_id, expected_lib_id])
        .await?;

    assert_eq!(maybe_lib.len(), 1);
    assert_eq!(maybe_lib[0].cell_hash(), expected_lib_id);
    Ok(())
}

#[tokio::test]
async fn test_get_lib_hashes_by_code() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = TonContractFactory::builder(&client).build().await?;

    let address = TonAddress::from_str("EQCqX53C_Th32Xg7UyrlqF0ypmePjljxG8edlwfT-1QpG3TB")?;

    let state = factory.get_latest_account_state(&address).await?;
    let code = BagOfCells::parse(&state.code)?.into_single_root()?;

    let hashes = assert_ok!(LibraryProvider::extract_library_hashes(&[code]));

    log::info!("{:?}", hashes);

    let expected_lib_id = TonHash::from([
        159, 49, 244, 244, 19, 163, 172, 203, 112, 108, 136, 150, 42, 198, 157, 89, 16, 59, 1, 58,
        10, 221, 207, 174, 237, 93, 215, 60, 24, 250, 152, 168,
    ]);

    assert_eq!(hashes.len(), 1);
    assert_eq!(expected_lib_id, hashes[0]);

    Ok(())
}
