use tonlib_client::contract::BlockchainLibraryProvider;
use tonlib_core::cell::BagOfCells;
use tonlib_core::TonHash;

mod common;

#[tokio::test]
async fn test_get_libs_by_hash() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;

    let expected_lib_id = TonHash::from([
        159, 49, 244, 244, 19, 163, 172, 203, 112, 108, 136, 150, 42, 198, 157, 89, 16, 59, 1, 58,
        10, 221, 207, 174, 237, 93, 215, 60, 24, 250, 152, 168,
    ]);
    log::info!("{:?}", expected_lib_id);

    let library_loader = BlockchainLibraryProvider::new(&client, None);
    let maybe_lib = library_loader
        .load_libraries(&[expected_lib_id, expected_lib_id])
        .await?;

    let boc = BagOfCells::from_root(maybe_lib[0].as_ref().clone())
        .serialize(true)
        .unwrap();
    log::info!("{}", hex::encode(boc));

    assert_eq!(maybe_lib.len(), 1);
    assert_eq!(maybe_lib[0].cell_hash(), expected_lib_id);
    Ok(())
}
