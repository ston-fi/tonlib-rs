use crate::cell::{ArcCell, BagOfCells, TonCellError};
use crate::tl::{MsgData, RawMessage};

pub trait RawMessageUtils {
    fn get_raw_data_cell(&self) -> Result<ArcCell, TonCellError>;
    // fn is_bounced(&self) -> bool;
}

impl RawMessageUtils for &RawMessage {
    fn get_raw_data_cell(&self) -> Result<ArcCell, TonCellError> {
        let msg_data = match &self.msg_data {
            MsgData::Raw { body, .. } => Ok(body.as_slice()),
            _ => Err(TonCellError::CellParserError(
                "Unsupported MsgData".to_string(),
            )),
        }?;

        let boc = BagOfCells::parse(msg_data)?;
        let cell = boc.single_root()?.clone();
        Ok(cell)
    }
}
