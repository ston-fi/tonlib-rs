use crate::cell::{ArcCell, CellBuilder, TonCellError};

pub(super) fn write_up_to_4_msgs(
    dst: &mut CellBuilder,
    msgs: &[ArcCell],
    msgs_modes: &[u8],
) -> Result<(), TonCellError> {
    validate_msgs_count(msgs, msgs_modes)?;
    for (msg, mode) in msgs.iter().zip(msgs_modes.iter()) {
        dst.store_u8(8, *mode)?;
        dst.store_reference(msg)?;
    }
    Ok(())
}

fn validate_msgs_count(msgs: &[ArcCell], msgs_modes: &[u8]) -> Result<(), TonCellError> {
    if msgs.len() > 4 || msgs_modes.len() != msgs.len() {
        let err_str = format!(
            "wrong msgs: modes_len={}, msgs_len={}, max_len=4",
            msgs_modes.len(),
            msgs.len()
        );
        Err(TonCellError::InvalidCellData(err_str))
    } else {
        Ok(())
    }
}
