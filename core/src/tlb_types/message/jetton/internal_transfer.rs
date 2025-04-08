use num_bigint::BigUint;

use crate::cell::ArcCell;
use crate::message::JETTON_INTERNAL_TRANSFER;
use crate::tlb_types::block::msg_address::MsgAddress;
use crate::tlb_types::primitives::either::EitherRef;
use crate::tlb_types::tlb::{TLBPrefix, TLB};

#[derive(Debug, Clone, PartialEq)]
pub struct JettonInternalTransferMessage {
    pub query_id: u64,
    pub amount: BigUint,
    pub from_address: MsgAddress,
    pub response_address: MsgAddress,
    pub fwd_amount: BigUint,
    pub either_forward_payload: EitherRef<ArcCell>,
}

impl TLB for JettonInternalTransferMessage {
    const PREFIX: TLBPrefix = TLBPrefix::new(32, JETTON_INTERNAL_TRANSFER as u64);

    fn read_definition(
        parser: &mut crate::cell::CellParser,
    ) -> Result<Self, crate::cell::TonCellError> {
        let query_id = parser.load_u64(64)?;
        let jetton_amount = parser.load_coins()?;
        let from_address = parser.load_msg_address()?;
        let response_address = parser.load_msg_address()?;
        let fwd_amount = parser.load_coins()?;
        let either_forward_payload = TLB::read(parser)?;

        parser.ensure_empty()?;

        let result = JettonInternalTransferMessage {
            query_id,
            amount: jetton_amount,
            from_address,
            response_address,
            fwd_amount,
            either_forward_payload,
        };

        Ok(result)
    }

    fn write_definition(
        &self,
        dst: &mut crate::cell::CellBuilder,
    ) -> Result<(), crate::cell::TonCellError> {
        dst.store_u64(64, self.query_id)?;
        dst.store_coins(&self.amount)?;
        self.from_address.write(dst)?;
        self.response_address.write(dst)?;
        dst.store_coins(&self.fwd_amount)?;
        self.either_forward_payload.write(dst)?;

        dst.build()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::Arc;

    use lazy_static::lazy_static;
    use num_bigint::BigUint;

    use super::JettonInternalTransferMessage;
    use crate::cell::Cell;
    use crate::message::TonMessageError;
    use crate::tlb_types::primitives::either::{EitherRef, EitherRefLayout};
    use crate::tlb_types::tlb::TLB;
    use crate::TonAddress;

    const JETTON_INTERNAL_TRANSFER_MSG : &str="b5ee9c720101020100aa0001af178d45190000005209ddeb9e440ee9390801e6ef228644c75beba08c8b8e2adf62f1e760e84861b5c33027f0433e19085713003cdde450c898eb7d74119171c55bec5e3cec1d090c36b86604fe0867c3210ae2501dcd65030100992593856180022a16a3164c4d5aa3133f3110ff10496e00ca8ac8abeffc5027e024d33480c3ea916f9f4a23003cdde450c898eb7d74119171c55bec5e3cec1d090c36b86604fe0867c3210ae250";
    const INTERNAL_TRANSFER_PAYLOAD: &str = "2593856180022A16A3164C4D5AA3133F3110FF10496E00CA8AC8ABEFFC5027E024D33480C3EA916F9F4A23003CDDE450C898EB7D74119171C55BEC5E3CEC1D090C36B86604FE0867C3210AE240";

    lazy_static! {
        static ref INTERNAL_TRANSFER_MESSAGE_CELL: Arc<Cell> = Arc::new(
            Cell::new(
                hex::decode(INTERNAL_TRANSFER_PAYLOAD).unwrap(),
                611,
                vec![],
                false,
            )
            .unwrap()
        );
        static ref EXPECTED_JETTON_INTERNAL_TRANSFER_MSG: JettonInternalTransferMessage =
            JettonInternalTransferMessage {
                query_id: 352352856990,
                amount: BigUint::from(1089377168u64),
                from_address: TonAddress::from_str(
                    "UQDzd5FDImOt9dBGRccVb7F487B0JDDa4ZgT-CGfDIQriSB-"
                )
                .unwrap()
                .to_msg_address(),
                response_address: TonAddress::from_str(
                    "UQDzd5FDImOt9dBGRccVb7F487B0JDDa4ZgT-CGfDIQriSB-",
                )
                .unwrap()
                .to_msg_address(),
                fwd_amount: BigUint::from(125000000u64),
                either_forward_payload: EitherRef {
                    value: INTERNAL_TRANSFER_MESSAGE_CELL.clone(),
                    layout: EitherRefLayout::ToRef,
                },
            };
    }

    #[test]
    fn test_jetton_internal_transfer_parser() -> Result<(), TonMessageError> {
        let result_jetton_internal_transfer_msg =
            JettonInternalTransferMessage::from_boc_hex(JETTON_INTERNAL_TRANSFER_MSG)?;

        let transfer_message_cell = Arc::new(Cell::new(
            hex::decode(INTERNAL_TRANSFER_PAYLOAD).unwrap(),
            611,
            vec![],
            false,
        )?);

        let expected_jetton_internal_transfer_msg = JettonInternalTransferMessage {
            query_id: 352352856990,
            amount: BigUint::from(1089377168u64),
            from_address: TonAddress::from_str("UQDzd5FDImOt9dBGRccVb7F487B0JDDa4ZgT-CGfDIQriSB-")
                .unwrap()
                .to_msg_address(),
            response_address: TonAddress::from_str(
                "UQDzd5FDImOt9dBGRccVb7F487B0JDDa4ZgT-CGfDIQriSB-",
            )
            .unwrap()
            .to_msg_address(),
            fwd_amount: BigUint::from(125000000u64),
            either_forward_payload: EitherRef {
                value: transfer_message_cell,
                layout: EitherRefLayout::ToRef,
            },
        };

        assert_eq!(
            expected_jetton_internal_transfer_msg,
            result_jetton_internal_transfer_msg
        );
        Ok(())
    }

    #[test]
    fn test_jetton_internal_transfer_builder() -> anyhow::Result<()> {
        let jetton_internal_transfer_msg = EXPECTED_JETTON_INTERNAL_TRANSFER_MSG.clone();
        let result_cell = jetton_internal_transfer_msg.to_cell()?;
        let result_boc_serialized = result_cell.to_boc(false)?;

        let expected_boc_serialized = hex::decode(JETTON_INTERNAL_TRANSFER_MSG)?;

        assert_eq!(expected_boc_serialized, result_boc_serialized);
        Ok(())
    }
}
