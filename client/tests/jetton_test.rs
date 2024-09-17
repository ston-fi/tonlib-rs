use sha2::{Digest, Sha256};
use tokio_test::assert_ok;
use tonlib_client::contract::{JettonMasterContract, TonContractFactory};
use tonlib_client::meta::{JettonMetaLoader, LoadMeta, MetaDataContent};
use tonlib_core::{TonAddress, TonHash};

mod common;

#[tokio::test]
async fn test_get_jetton_content_uri() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR".parse()
    )); // Moon jetton
    let res = assert_ok!(contract.get_jetton_data().await);
    assert_eq!(
        res.content,
        MetaDataContent::External {
            uri: "https://tarantini.dev/ston/moon.json".to_string()
        }
    );
    let meta_loader = assert_ok!(JettonMetaLoader::default());
    let content_res = assert_ok!(meta_loader.load(&res.content).await);
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("MOON"));
    assert_eq!(content_res.decimals.unwrap(), 0x9);
}

#[tokio::test]
async fn test_get_jetton_content_internal_uri() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQDCJL0iQHofcBBvFBHdVG233Ri2V4kCNFgfRT-gqAd3Oc86".parse()
    )); // Fanzee jetton
    let res = assert_ok!(contract.get_jetton_data().await);
    let meta_loader = assert_ok!(JettonMetaLoader::default());
    let content_res = assert_ok!(meta_loader.load(&res.content).await);
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("FNZ"));
    assert_eq!(content_res.decimals.unwrap(), 0x9);
}

#[tokio::test]
async fn test_get_jetton_content_internal_uri_jusdt() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQBynBO23ywHy_CgarY9NK9FTz0yDsG82PtcbSTQgGoXwiuA".parse()
    )); // jUSDT jetton
    let res = assert_ok!(contract.get_jetton_data().await);
    let meta_loader = assert_ok!(JettonMetaLoader::default());
    let content_res = assert_ok!(meta_loader.load(&res.content).await);
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("jUSDT"));
    assert_eq!(content_res.decimals, Some(6));
}

#[tokio::test]
async fn test_get_jetton_content_empty_external_meta() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQD-J6UqYQezuUm6SlPDnHwTxXqo4uHys_fle_zKvM5nYJkA".parse()
    ));
    let res = assert_ok!(contract.get_jetton_data().await);
    let meta_loader = assert_ok!(JettonMetaLoader::default());
    let content_res = assert_ok!(meta_loader.load(&res.content).await);
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("BLKC"));
    assert_eq!(content_res.decimals, Some(8));
}

// this test is ignored due restrictions of cloudflare-ipfs.com
#[tokio::test]
#[ignore]
async fn test_get_jetton_content_ipfs_uri() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQD0vdSA_NedR9uvbgN9EikRX-suesDxGeFg69XQMavfLqIw".parse()
    )); // BOLT jetton
    let res = assert_ok!(contract.get_jetton_data().await);
    let meta_loader = assert_ok!(JettonMetaLoader::default());
    let content_res = assert_ok!(meta_loader.load(&res.content).await);
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("BOLT"));
    log::info!("{:?}", content_res);
    log::info!("{:?}", content_res.image_data);
    assert_eq!(content_res.decimals.unwrap(), 0x9);

    log::info!("{:?}", res);
}

#[tokio::test]
async fn test_get_semi_chain_layout_jetton_content() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQB-MPwrd1G6WKNkLz_VnV6WqBDd142KMQv-g1O-8QUA3728".parse()
    )); // jUSDC jetton
    let res = assert_ok!(contract.get_jetton_data().await);
    let meta_loader = assert_ok!(JettonMetaLoader::default());
    let content_res = assert_ok!(meta_loader.load(&res.content).await);
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("jUSDC"));
    assert_eq!(
        content_res.name.as_ref().unwrap(),
        &String::from("USD Coin")
    );
    assert_eq!(content_res.decimals.unwrap(), 0x6);
}

#[tokio::test]
async fn test_get_wallet_address() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR".parse()
    ));
    let owner_address = assert_ok!(TonAddress::from_base64_url(
        "EQB2BtXDXaQuIcMYW7JEWhHmwHfPPwa-eoCdefiAxOhU3pQg"
    ));
    let wallet_address = assert_ok!(contract.get_wallet_address(&owner_address).await);
    assert_eq!(
        "EQCGY3OVLtD9KRcOsP2ldQDtuY0FMzV7wPoxjrFbayBXc23c",
        wallet_address.to_base64_std()
    );
}

#[tokio::test]
async fn test_get_jetton_data_invalid_utf8_sequence() {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQDX__KZ7A--poP3Newpo_zx4tQ-yl9yzRwlmg_vifxMEA8m".parse()
    ));
    let res = assert_ok!(contract.get_jetton_data().await);
    log::info!("DATA: {:?}", res);
    let meta_loader = assert_ok!(JettonMetaLoader::default());
    let content_res = assert_ok!(meta_loader.load(&res.content).await);
    assert_eq!(
        content_res.symbol.as_ref().unwrap(),
        &String::from("DuRove's")
    );
    assert_eq!(content_res.decimals.unwrap(), 0x9);

    let contract = factory.get_contract(&assert_ok!(
        "EQDoEAodkem9PJdk3W1mqjnkvRphNaWu0glIRzxQBNZuOIbP".parse()
    ));
    let res = assert_ok!(contract.get_jetton_data().await);
    log::info!("DATA: {:?}", res);
    let meta_loader = assert_ok!(JettonMetaLoader::default());
    let content_res = assert_ok!(meta_loader.load(&res.content).await);
    assert_eq!(content_res.symbol.as_ref().unwrap(), &String::from("TFH"));
    assert_eq!(content_res.decimals.unwrap(), 0x9);
}

#[tokio::test]
async fn test_jetton_image_data() -> anyhow::Result<()> {
    common::init_logging();
    let client = common::new_mainnet_client().await;
    let factory = assert_ok!(TonContractFactory::builder(&client).build().await);
    let contract = factory.get_contract(&assert_ok!(
        "EQCTzzFI_I9OmQu5CfiK1k2TIV8ZvMtIWRkMjHdRrpfhJuBX".parse()
    ));

    let jetton_data = assert_ok!(contract.get_jetton_data().await);

    let meta_loader = assert_ok!(JettonMetaLoader::default());
    let content_res = assert_ok!(meta_loader.load(&jetton_data.content).await);

    const TARGET_IMAGE_HASH: TonHash = [
        45, 186, 67, 118, 224, 166, 76, 84, 0, 203, 69, 175, 47, 34, 164, 184, 36, 229, 51, 193,
        17, 18, 84, 70, 179, 240, 137, 163, 42, 147, 119, 220,
    ];
    let mut hasher: Sha256 = Sha256::new();
    hasher.update(content_res.image_data.unwrap());
    let img_hash = hasher.finalize()[..].to_vec();
    assert_eq!(TARGET_IMAGE_HASH.to_vec(), img_hash);

    Ok(())
}
