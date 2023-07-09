use tonlib::cell::BagOfCells;
use tonlib::client::TonFunctions;

mod common;

#[tokio::test]
async fn test_config_works() -> anyhow::Result<()> {
    common::init_logging();
    let client = &common::new_test_client().await?;
    let info = client.get_config_param(0u32, 34u32).await?;
    let config_data = info.config.bytes;
    let bag = BagOfCells::parse(config_data.as_slice())?;
    let config_cell = bag.single_root()?;
    let mut parser = config_cell.parser();
    let n = parser.load_u8(8)?;
    assert!(n == 0x12u8);
    Ok(())
}
