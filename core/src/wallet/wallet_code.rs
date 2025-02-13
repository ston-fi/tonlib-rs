use std::collections::HashMap;

use lazy_static::lazy_static;

use crate::cell::{ArcCell, BagOfCells};
use crate::wallet::wallet_version::WalletVersion;

macro_rules! load_code {
    ($path:expr) => {
        BagOfCells::parse_base64(include_str!($path))
            .unwrap()
            .into_single_root()
            .unwrap()
    };
}

lazy_static! {
    #[allow(clippy::all)]
    pub(super) static ref WALLET_CODE_BY_VERSION: HashMap<WalletVersion, ArcCell> =
        HashMap::from([
            (WalletVersion::V1R1, load_code!("../../resources/wallet/wallet_v1r1.code")),
            (WalletVersion::V1R2, load_code!("../../resources/wallet/wallet_v1r2.code")),
            (WalletVersion::V1R3, load_code!("../../resources/wallet/wallet_v1r3.code")),
            (WalletVersion::V2R1, load_code!("../../resources/wallet/wallet_v2r1.code")),
            (WalletVersion::V2R2, load_code!("../../resources/wallet/wallet_v2r2.code")),
            (WalletVersion::V3R1, load_code!("../../resources/wallet/wallet_v3r1.code")),
            (WalletVersion::V3R2, load_code!("../../resources/wallet/wallet_v3r2.code")),
            (WalletVersion::V4R1, load_code!("../../resources/wallet/wallet_v4r1.code")),
            (WalletVersion::V4R2, load_code!("../../resources/wallet/wallet_v4r2.code")),
            (WalletVersion::V5R1, load_code!("../../resources/wallet/wallet_v5.code")),
            (WalletVersion::HighloadV1R1, load_code!("../../resources/wallet/highload_v1r1.code")),
            (WalletVersion::HighloadV1R2, load_code!("../../resources/wallet/highload_v1r2.code")),
            (WalletVersion::HighloadV2, load_code!("../../resources/wallet/highload_v2.code")),
            (WalletVersion::HighloadV2R1, load_code!("../../resources/wallet/highload_v2r1.code")),
            (WalletVersion::HighloadV2R2, load_code!("../../resources/wallet/highload_v2r2.code")),
        ]);
}
