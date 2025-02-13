use crate::cell::{ArcCell, CellBuilder, TonCellError};

pub(super) fn write_up_to_4_msgs(
    dst: &mut CellBuilder,
    msgs: &[ArcCell],
    msgs_modes: &[u8],
) -> Result<(), TonCellError> {
    validate_msgs_count(msgs, msgs_modes, 4)?;
    for (msg, mode) in msgs.iter().zip(msgs_modes.iter()) {
        dst.store_u8(8, *mode)?;
        dst.store_reference(msg)?;
    }
    Ok(())
}

pub(super) fn validate_msgs_count(
    msgs: &[ArcCell],
    msgs_modes: &[u8],
    max_cnt: usize,
) -> Result<(), TonCellError> {
    if msgs.len() > max_cnt || msgs_modes.len() != msgs.len() {
        let err_str = format!(
            "wrong msgs: modes_len={}, msgs_len={}, max_len={max_cnt}",
            msgs_modes.len(),
            msgs.len()
        );
        Err(TonCellError::InvalidCellData(err_str))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use tokio_test::{assert_err, assert_ok};

    use super::*;

    #[test]
    fn test_write_up_to_4_msgs() -> anyhow::Result<()> {
        let mut builder = CellBuilder::new();
        let msgs = vec![ArcCell::default(), ArcCell::default()];
        let msgs_modes = vec![1, 2];
        assert_ok!(write_up_to_4_msgs(&mut builder, &msgs, &msgs_modes));

        let mut builder = CellBuilder::new();
        let msgs = vec![
            ArcCell::default(),
            ArcCell::default(),
            ArcCell::default(),
            ArcCell::default(),
        ];
        let msgs_modes = vec![1, 2, 3, 4];
        assert_ok!(write_up_to_4_msgs(&mut builder, &msgs, &msgs_modes));

        let mut builder = CellBuilder::new();
        let msgs = vec![
            ArcCell::default(),
            ArcCell::default(),
            ArcCell::default(),
            ArcCell::default(),
            ArcCell::default(),
        ];
        let msgs_modes = vec![1, 2, 3, 4, 5];
        assert_err!(write_up_to_4_msgs(&mut builder, &msgs, &msgs_modes));

        Ok(())
    }
}
