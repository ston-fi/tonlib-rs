use thiserror::Error;

use crate::tlb_types::tlb::TLBPrefix;
use crate::types::TonHashParseError;

#[derive(Error, Debug)]
pub enum TonCellError {
    #[error("Bag of cells deserialization error ({0})")]
    BagOfCellsDeserializationError(String),

    #[error("Bag of cells serialization error ({0})")]
    BagOfCellsSerializationError(String),

    #[error("Cell builder error ({0})")]
    CellBuilderError(String),

    #[error("Cell parser error ({0})")]
    CellParserError(String),

    #[error("Internal error ({0})")]
    InternalError(String),

    #[error("Invalid index (Index: {idx}, reference count: {ref_count})")]
    InvalidIndex { idx: usize, ref_count: usize },

    #[error("Invalid address type (Type: {0})")]
    InvalidAddressType(u8),

    #[error("Invalid cell type for exotic cell (Type: {0:?})")]
    InvalidExoticCellType(Option<u8>),

    #[error("Invalid exotic cell data (({0})")]
    InvalidExoticCellData(String),

    #[error("Invalid cell data ({0})")]
    InvalidCellData(String),

    #[error("Invalid input error ({0})")]
    InvalidInput(String),

    #[error("Invalid TLB prefix: (expected: {expected_prefix}, actual {actual_prefix}, expected bit_len: {expected_bit_len}, remaining bit_len: {remaining_bit_len}")]
    InvalidTLBPrefix {
        expected_prefix: u64,
        actual_prefix: u64,
        expected_bit_len: usize,
        remaining_bit_len: usize,
    },

    #[error(
        "Non-empty reader (Remaining bits: {remaining_bits}, Remaining refs: {remaining_refs})"
    )]
    NonEmptyReader {
        remaining_bits: usize,
        remaining_refs: usize,
    },
    #[error("TonHash parse error ({0})")]
    TonHashParseError(#[from] TonHashParseError),

    #[error("{0}")]
    IO(#[from] std::io::Error),
}

pub trait MapTonCellError<R, E>
where
    E: std::error::Error,
{
    fn map_boc_deserialization_error(self) -> Result<R, TonCellError>;

    fn map_boc_serialization_error(self) -> Result<R, TonCellError>;

    fn map_cell_builder_error(self) -> Result<R, TonCellError>;

    fn map_cell_parser_error(self) -> Result<R, TonCellError>;
}

impl<R, E> MapTonCellError<R, E> for Result<R, E>
where
    E: std::error::Error,
{
    fn map_boc_serialization_error(self) -> Result<R, TonCellError> {
        self.map_err(|e| TonCellError::boc_serialization_error(e))
    }

    fn map_boc_deserialization_error(self) -> Result<R, TonCellError> {
        self.map_err(|e| TonCellError::boc_deserialization_error(e))
    }

    fn map_cell_builder_error(self) -> Result<R, TonCellError> {
        self.map_err(|e| TonCellError::cell_builder_error(e))
    }

    fn map_cell_parser_error(self) -> Result<R, TonCellError> {
        self.map_err(|e| TonCellError::cell_parser_error(e))
    }
}

impl TonCellError {
    pub fn boc_serialization_error<T>(e: T) -> TonCellError
    where
        T: ToString,
    {
        TonCellError::BagOfCellsSerializationError(format!(
            "BoC serialization error: {}",
            e.to_string()
        ))
    }

    pub fn boc_deserialization_error<T>(e: T) -> TonCellError
    where
        T: ToString,
    {
        TonCellError::BagOfCellsDeserializationError(format!(
            "BoC deserialization error: {}",
            e.to_string()
        ))
    }

    pub fn cell_builder_error<T>(e: T) -> TonCellError
    where
        T: ToString,
    {
        TonCellError::CellBuilderError(format!("Cell builder error: {}", e.to_string()))
    }

    pub fn cell_parser_error<T>(e: T) -> TonCellError
    where
        T: ToString,
    {
        TonCellError::CellParserError(format!("Cell parser error: {}", e.to_string()))
    }

    pub fn tlb_prefix_error(
        expected_prefix: TLBPrefix,
        actual_prefix: u64,
        bit_len_remaining: usize,
    ) -> TonCellError {
        TonCellError::InvalidTLBPrefix {
            expected_prefix: expected_prefix.value,
            actual_prefix,
            expected_bit_len: expected_prefix.bit_len,
            remaining_bit_len: bit_len_remaining,
        }
    }
}
