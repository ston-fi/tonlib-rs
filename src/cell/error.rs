use thiserror::Error;

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

    #[error("Non-empty reader (Remaining bits: {0})")]
    NonEmptyReader(usize),
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
}
