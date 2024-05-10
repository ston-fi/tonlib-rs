use std::ffi::CString;
use std::io;
use std::sync::Arc;

use hmac::digest::InvalidLength;
use num_bigint::BigInt;
use pbkdf2::password_hash::Error;
use reqwest::StatusCode;
use tokio_test::assert_ok;
use tonlib::address::{TonAddress, TonAddressParseError};
use tonlib::cell::{CellBuilder, CellSlice, TonCellError};
use tonlib::client::TonClientError;
use tonlib::contract::TonContractError;
use tonlib::message::TonMessageError;
use tonlib::meta::{IpfsLoaderError, MetaDataContent, MetaLoaderError};
use tonlib::mnemonic::MnemonicError;
use tonlib::tl::{
    InternalTransactionIdParseError, TlError, TonResultDiscriminants, TvmCell,
    TvmStackEntry as TlTvmStackEntry, TvmStackError,
};
use tonlib::types::TvmStackEntry;

mod common;

// This test is used to demonstrate all available errors.
#[test]
#[ignore]
fn test_all_error_output() {
    test_ton_address_error_output();
    log::info!("-------------------------------------------------------------\n");
    test_ton_cell_error_output();
    log::info!("-------------------------------------------------------------\n");
    test_ton_client_error_output();
    log::info!("-------------------------------------------------------------\n");
    test_ton_contract_error_output();
    log::info!("-------------------------------------------------------------\n");
    test_ton_message_error_output();
    log::info!("-------------------------------------------------------------\n");
    test_meta_loader_error_output();
    log::info!("-------------------------------------------------------------\n");
    test_message_error_output();
    log::info!("-------------------------------------------------------------\n");
    test_tvm_stack_error_output();
    log::info!("-------------------------------------------------------------\n");
    test_tl_error_output();
    log::info!("-------------------------------------------------------------\n");
    test_internal_txid_parse_error_output();
}

#[test]
#[ignore]
fn test_ton_address_error_output() {
    common::init_logging();
    log::error!(
        "{}",
        TonAddressParseError::new(
            "EQQLKJGBEolgn2nl1;1`ln4141jl4n1n421n24142oololl",
            "Invalid base64 address",
        )
    );
}

#[test]
#[ignore]
fn test_ton_cell_error_output() {
    common::init_logging();
    log::error!(
        "{}",
        TonCellError::BagOfCellsDeserializationError("Some error message".to_string())
    );
    log::error!(
        "{}",
        TonCellError::BagOfCellsSerializationError("Some error message".to_string())
    );
    log::error!(
        "{}",
        TonCellError::CellBuilderError("Some error message".to_string())
    );
    log::error!(
        "{}",
        TonCellError::CellParserError("Some error message".to_string())
    );
    log::error!(
        "{}",
        TonCellError::InternalError("Some error message".to_string())
    );
    log::error!(
        "{}",
        TonCellError::InvalidIndex {
            idx: 4,
            ref_count: 5
        },
    );
    log::error!("{}", TonCellError::InvalidAddressType(200));
    log::error!("{}", TonCellError::NonEmptyReader(300));
}

#[test]
#[ignore]
fn test_ton_client_error_output() {
    common::init_logging();
    log::error!(
        "{}",
        TonClientError::InternalError("Some error message".to_string())
    );
    log::error!(
        "{}",
        TonClientError::TonlibError {
            method: "some_get_method",
            code: 300,
            message: "Some error message".to_string(),
        }
    );
    log::error!(
        "{}",
        TonClientError::UnexpectedTonResult {
            actual: TonResultDiscriminants::BlocksMasterchainInfo,
            expected: TonResultDiscriminants::BlocksTransactions,
        }
    );
    log::error!(
        "{}",
        TonClientError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            "Some error message"
        ))
    );
    #[allow(invalid_from_utf8)]
    let utf8_error = std::str::from_utf8(&[0xC3, 0x28]).unwrap_err();
    log::error!(
        "{}",
        TonClientError::TlError(TlError::Utf8Error(utf8_error))
    )
}

#[test]
#[ignore]
fn test_ton_contract_error_output() {
    common::init_logging();
    log::error!(
        "{}",
        TonContractError::CellError {
            method: "some_get_method".to_string(),
            address: TonAddress::null(),
            error: TonCellError::InvalidIndex {
                idx: 4,
                ref_count: 5
            }
        }
    );
    #[allow(invalid_from_utf8)]
    let utf8_error = std::str::from_utf8(&[0xC3, 0x28]).unwrap_err();

    log::error!(
        "{}",
        TonContractError::ClientError(TonClientError::TlError(TlError::Utf8Error(utf8_error)))
    );

    log::error!(
        "{}",
        TonContractError::ClientError(TonClientError::TonlibError {
            method: "some_get_method",
            code: 300,
            message: "Some error message".to_string(),
        })
    );

    log::error!(
        "{}",
        TonContractError::IllegalArgument("Some error message".to_string())
    );

    log::error!(
        "{}",
        TonContractError::InvalidMethodResultStackSize {
            method: "some_get_method".to_string(),
            address: TonAddress::null(),
            actual: 10,
            expected: 2,
        }
    );

    log::error!(
        "{}",
        TonContractError::MethodResultStackError {
            method: "some_get_method".into(),
            address: TonAddress::null(),
            error: TvmStackError::TonCellError(TonCellError::BagOfCellsDeserializationError(
                "Some error message".to_string()
            )),
        }
    );

    let cell =
        assert_ok!(assert_ok!(CellBuilder::new().store_address(&TonAddress::null())).build());
    log::error!(
        "{}",
        TonContractError::TvmRunError {
            method: "some_get_method".into(),
            gas_used: 300,
            stack: vec![
                TvmStackEntry::Slice(assert_ok!(CellSlice::full_cell(cell.clone()))),
                TvmStackEntry::Cell(Arc::new(cell)),
                TvmStackEntry::Int257(BigInt::from(1234566789)),
            ],
            exit_code: -123,
            vm_log: None,
            missing_library: None,
            address: TonAddress::null(),
        }
    );
}

#[test]
#[ignore]
fn test_ton_message_error_output() {
    common::init_logging();
    log::error!(
        "{}",
        TonMessageError::NaclCryptographicError("Some error message".to_string())
    );
    log::error!("{}", TonMessageError::ForwardTonAmountIsNegative);

    log::error!(
        "{}",
        TonMessageError::TonCellError(TonCellError::BagOfCellsDeserializationError(
            "Some error message".to_string()
        ))
    );
}

#[test]
#[ignore]
fn test_meta_loader_error_output() {
    common::init_logging();
    log::error!(
        "{}",
        MetaLoaderError::ContentLayoutUnsupported(MetaDataContent::External {
            uri: "some_uri.xx".to_string()
        })
    );
    log::error!(
        "{}",
        MetaLoaderError::LoadMetaDataFailed {
            uri: "some_uri.xx".to_string(),
            status: StatusCode::BAD_GATEWAY
        }
    );

    log::error!(
        "{}",
        MetaLoaderError::IpfsLoaderError(IpfsLoaderError::IpfsLoadObjectFailed {
            path: "some_uri.xx".to_string(),
            status: StatusCode::BAD_GATEWAY,
            message: "Some error message".to_string()
        })
    );
    let json_str = "";
    let serde_json_err = serde_json::from_str::<&str>(json_str).unwrap_err();
    log::error!("{}", MetaLoaderError::SerdeJsonError(serde_json_err));
}

#[test]
#[ignore]
fn test_message_error_output() {
    common::init_logging();
    log::error!("{}", MnemonicError::UnexpectedWordCount(300),);
    log::error!("{}", MnemonicError::InvalidWord("Aaaaaa".to_string()),);
    log::error!("{}", MnemonicError::InvalidFirstByte(0xFF),);
    log::error!(
        "{}",
        MnemonicError::InvalidPasswordlessMenmonicFirstByte(0xFF),
    );

    log::error!("{}", MnemonicError::PasswordHashError(Error::Algorithm),);

    log::error!("{}", MnemonicError::ShaDigestLengthInvalid(InvalidLength),);
}

#[test]
#[ignore]
fn test_tvm_stack_error_output() {
    common::init_logging();
    let cell_entry = TlTvmStackEntry::Cell {
        cell: TvmCell { bytes: vec![0x00] },
    };
    log::error!(
        "{}",
        TvmStackError::StringConversion {
            e: cell_entry.clone(),
            index: 1
        }
    );
    log::error!(
        "{}",
        TvmStackError::I32Conversion {
            e: cell_entry.clone(),
            index: 1
        }
    );
    log::error!(
        "{}",
        TvmStackError::I64Conversion {
            e: cell_entry.clone(),
            index: 1
        },
    );
    log::error!(
        "{}",
        TvmStackError::BigUintConversion {
            e: cell_entry.clone(),
            index: 1
        },
    );
    log::error!(
        "{}",
        TvmStackError::BigIntConversion {
            e: cell_entry.clone(),
            index: 1
        },
    );
    log::error!(
        "{}",
        TvmStackError::BoCConversion {
            e: cell_entry,
            index: 1
        },
    );
    log::error!(
        "{}",
        TvmStackError::InvalidTvmStackIndex { index: 10, len: 1 },
    );
    let cell_error = TonCellError::InternalError("Some error".to_string());
    log::error!("{}", TvmStackError::TonCellError(cell_error))
}

#[test]
#[ignore]
fn test_tl_error_output() {
    common::init_logging();
    #[allow(invalid_from_utf8)]
    let utf8_error = std::str::from_utf8(&[0xC3, 0x28]).unwrap_err();

    log::error!("{}", TlError::Utf8Error(utf8_error));

    let json_str = "";
    let serde_json_err = serde_json::from_str::<&str>(json_str).unwrap_err();

    log::error!("{}", TlError::SerdeJsonError(serde_json_err));
    let nul_error = CString::new(vec![0x00, 0x00, 0x00]).err().unwrap();
    log::error!("{}", TlError::NulError(nul_error));
}

#[test]
#[ignore]
fn test_internal_txid_parse_error_output() {
    common::init_logging();
    log::error!(
        "{}",
        InternalTransactionIdParseError::new("bad txid".to_string(), "bad message".to_string())
    );
}
