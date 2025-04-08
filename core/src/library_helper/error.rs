use thiserror::Error;

use crate::cell::TonCellError;
use crate::types::TonHashParseError;

#[derive(Error, Debug)]
pub enum TonLibraryError {
    #[error("{0}")]
    TonClientError(String),

    #[error("{0}")]
    TonCellError(#[from] TonCellError),

    #[error("{0}")]
    TonHashParseError(#[from] TonHashParseError),

    #[error("Library not found for {0}")]
    LibraryNotFound(String),

    #[error("Expected exactly one library, but got multiple")]
    MultipleLibrariesReturned,

    #[error("Getting library by mc_seqno is not supported")]
    SeqnoNotSupported,
}
