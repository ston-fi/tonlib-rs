use num_bigint::BigUint;
use num_traits::FromPrimitive;
use tonlib::address::TonAddress;
use tonlib::cell::BagOfCells;
use tonlib::message::TransferMessage;
use tonlib::mnemonic::KeyPair;
use tonlib::wallet::{TonWallet, WalletVersion};

const BOC1: &str = "b5ee9c720101030100fa00018a5eec183e69f564e9f47bdc4b5d1f64397659ec657e2fc159c7f3667768bd198c66efa94dd93751b7c1dc70c7b5aea163cb5b80b62029b33f2cc5508f6170e20f65cf681700010168620051166e900913355b08d97ce952f14c5c4952c4ad1a39a3b937f11cc79d6a03dea017d78400000000000000000000000000010200f24d696e650065cf6b9664ab5d46f8216912274718ac63d1cb7d1aa3e17fb820ec501b982650c0d3533e71fe83a0dec41fc37d10d933c7744b3b71c07269fd5a3f5f65ef3f458d2e7b048dc905febb6b33d632f049ebbeb4f00d71fe83a0dec41fc37d10d933c7744b3b71c07269fd5a3f5f65ef3f458d2e7b04";
const BOC2: &str = "b5ee9c720101030100fa00018a3b500958d0886596067aecaaf1b56c5ab91791bcbb79df1dc3acdf9720bccc1854e9b77abfb36e4a3b91e04d3aa91d2832c26b5aceffd8cb073f366e16e6a70965cf681702010168620051166e900913355b08d97ce952f14c5c4952c4ad1a39a3b937f11cc79d6a03dea017d78400000000000000000000000000010200f24d696e650065cf6b9664ab5d46f8216912274718ac63d1cb7d1aa3e17fb820ec501b982650c0d3533e71fe83a0dec41fc37d10d933c7744b3b71c07269fd5a3f5f65ef3f458d2e7b048dc905febb6b33d632f049ebbeb4f00d71fe83a0dec41fc37d10d933c7744b3b71c07269fd5a3f5f65ef3f458d2e7b04";
const MY_BOC: &str = "b5ee9c720102030100010300019c57c69ee6070fd9c6fe9f23194308284d056f81fb249c9da912fe4bf6831df43b24ad7ba4df6b2931ed78df44602de67963641961ae6641789e14039e915f0a050000002065d136d4000017b700030101686200113a1e74f57f143df5e50ab5f62be18866c7dc2ad3ca0b38c66e44d594412a112017d78400000000000000000000000000010200f24d696e650065d13a446cd676d146bb2c90a94dd36669cc15e4cbe447b34a65abd23056152673ae3a3ad0d79dd178101212be72df203ee91bf1f6a76b3cceb4669166293fc81452a29ecf3ff415a1c129bae5253d46a8ac3172d0d79dd178101212be72df203ee91bf1f6a76b3cceb4669166293fc81452a29e";
#[tokio::test]
async fn test_cell_content() -> anyhow::Result<()> {
    let boc1 = BagOfCells::parse_hex(BOC1)?;
    let boc2 = BagOfCells::parse_hex(BOC2)?;
    let my_boc = BagOfCells::parse_hex(MY_BOC)?;
    println!("{:?}", boc1);
    println!("{:?}", boc2);
    println!("{:?}", my_boc);
    Ok(())
}

fn print_cell_info(name: &str, boc: &BagOfCells) -> anyhow::Result<()> {
    println!("{}: {:?}", name, boc);
    let mut parser = boc.single_root()?.parser();
    let sig = parser.load_bytes(64)?;
    // let version = parser.
    Ok(())
}

#[tokio::test]
async fn test_build_message() -> anyhow::Result<()> {
    let transfer1 = TransferMessage::new(&TonAddress::NULL, &BigUint::from_u32(100500).unwrap())
        .build()?
        .to_arc();
    let transfer2 = TransferMessage::new(&TonAddress::NULL, &BigUint::from_u32(200500).unwrap())
        .build()?
        .to_arc();
    let keypair: KeyPair = None.unwrap();
    let wallet = TonWallet::derive(0, WalletVersion::V4R2, &keypair, None)?;
    let msg = wallet.create_external_message(0, 0, vec![transfer1], false)?;
    let x = vec![100500, 200500];
    let transfers: Vec<_> = x
        .iter()
        .map(|v| {
            TransferMessage::new(&TonAddress::NULL, &BigUint::from_u32(*v).unwrap())
                .build()
                .unwrap()
                .to_arc()
        })
        .collect();
    let msg1 = wallet.create_external_message(0, 0, transfers, false)?;
    Ok(())
}
