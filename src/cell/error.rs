use thiserror::Error;

#[derive(Error, Debug)]
pub enum TonCellError {
    #[error("Bag of cells deserialization error: {msg}")]
    BagOfCellsDeserializationError { msg: String },

    #[error("Bag of cells serialization error: {msg}")]
    BagOfCellsSerializationError { msg: String },

    #[error("Cell builder error: {msg}")]
    CellBuilderError { msg: String },

    #[error("Cell parser error: {msg}")]
    CellParserError { msg: String },

    #[error("Internal error: {msg}")]
    InternalError { msg: String },

    #[error("Invalid index: {idx}, Cell contains {ref_count} references")]
    InvalidIndex { idx: usize, ref_count: usize },

    #[error("Invalid address type: {tp}")]
    InvalidAddressType { tp: u8 },

    #[error("Reader must be empty but there are {remaining_bits} remaining bits")]
    NonEmptyReader { remaining_bits: usize },
}

pub(crate) trait MapTonCellError<R, E>
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
    pub(crate) fn boc_serialization_error<T>(e: T) -> TonCellError
    where
        T: ToString,
    {
        TonCellError::BagOfCellsSerializationError {
            msg: format!("BoC serialization error: {}", e.to_string()),
        }
    }

    pub(crate) fn boc_deserialization_error<T>(e: T) -> TonCellError
    where
        T: ToString,
    {
        TonCellError::BagOfCellsDeserializationError {
            msg: format!("BoC deserialization error: {}", e.to_string()),
        }
    }

    pub(crate) fn cell_builder_error<T>(e: T) -> TonCellError
    where
        T: ToString,
    {
        TonCellError::CellBuilderError {
            msg: format!("Cell builder error: {}", e.to_string()),
        }
    }

    pub(crate) fn cell_parser_error<T>(e: T) -> TonCellError
    where
        T: ToString,
    {
        TonCellError::CellParserError {
            msg: format!("Cell parser error: {}", e.to_string()),
        }
    }
}
