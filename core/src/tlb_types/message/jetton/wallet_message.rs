use super::{JettonInternalTransferMessage, JettonTransferMessage};
use crate::cell::TonCellError;
use crate::message::{JETTON_INTERNAL_TRANSFER, JETTON_TRANSFER};
use crate::tlb_types::traits::TLBObject;

#[derive(Clone, Debug, PartialEq)]
pub enum JettonWalletMessage {
    JettonTransfer(JettonTransferMessage),
    JettonInternalTransfer(JettonInternalTransferMessage),
}

impl TLBObject for JettonWalletMessage {
    fn read(parser: &mut crate::cell::CellParser) -> Result<Self, crate::cell::TonCellError> {
        let opcode = parser.load_u32(32)?;
        // mv parser back
        parser.seek(-32)?;

        match opcode {
            JETTON_TRANSFER => {
                let jetton_transfer = TLBObject::read(parser)?;
                Ok(JettonWalletMessage::JettonTransfer(jetton_transfer))
            }
            JETTON_INTERNAL_TRANSFER => {
                let jetton_internal_transfer = TLBObject::read(parser)?;
                Ok(JettonWalletMessage::JettonInternalTransfer(
                    jetton_internal_transfer,
                ))
            }
            _ => Err(TonCellError::CellParserError(format!(
                "JettonWalletMessage: unsupported opcode: {opcode}"
            ))),
        }
    }

    fn write_to(
        &self,
        dst: &mut crate::cell::CellBuilder,
    ) -> Result<(), crate::cell::TonCellError> {
        match self {
            Self::JettonTransfer(jetton_transfer) => jetton_transfer.write_to(dst)?,
            Self::JettonInternalTransfer(jetton_internal_transfer) => {
                jetton_internal_transfer.write_to(dst)?
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::Arc;

    use lazy_static::lazy_static;
    use num_bigint::BigUint;

    use super::JettonTransferMessage;
    use crate::cell::Cell;
    use crate::message::TonMessageError;
    use crate::tlb_types::message::jetton::{JettonInternalTransferMessage, JettonWalletMessage};
    use crate::tlb_types::primitives::either::{EitherRef, EitherRefLayout};
    use crate::tlb_types::traits::TLBObject;
    use crate::TonAddress;

    const JETTON_TRANSFER_MSG : &str="b5ee9c720101020100a800016d0f8a7ea5001f5512dab844d643b9aca00800ef3b9902a271b2a01c8938a523cfe24e71847aaeb6a620001ed44a77ac0e709c1033428f030100d7259385618009dd924373a9aad41b28cec02da9384d67363af2034fc2a7ccc067e28d4110de86e66deb002365dfa32dfd419308ebdf35e0f6ba7c42534bbb5dab5e89e28ea3e0455cc2d2f00257a672371a90e149b7d25864dbfd44827cc1e8a30df1b1e0c4338502ade2ad96";
    const TRANSFER_PAYLOAD: &str = "259385618009DD924373A9AAD41B28CEC02DA9384D67363AF2034FC2A7CCC067E28D4110DE86E66DEB002365DFA32DFD419308EBDF35E0F6BA7C42534BBB5DAB5E89E28EA3E0455CC2D2F00257A672371A90E149B7D25864DBFD44827CC1E8A30DF1B1E0C4338502ADE2AD94";

    lazy_static! {
        static ref TRANSFER_MESSAGE_CELL: Arc<Cell> = Arc::new(
            Cell::new(hex::decode(TRANSFER_PAYLOAD).unwrap(), 862, vec![], false,).unwrap()
        );
        static ref EXPECTED_JETTON_TRANSFER_MSG: JettonTransferMessage = JettonTransferMessage {
            query_id: 8819263745311958,
            amount: BigUint::from(1000000000u64),
            destination: TonAddress::from_str("EQB3ncyBUTjZUA5EnFKR5_EnOMI9V1tTEAAPaiU71gc4TiUt")
                .unwrap()
                .to_msg_address(),
            response_destination: TonAddress::from_str(
                "EQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM9c",
            )
            .unwrap()
            .to_msg_address(),
            custom_payload: None,
            forward_ton_amount: BigUint::from(215000000u64),
            forward_payload: EitherRef {
                value: TRANSFER_MESSAGE_CELL.clone(),
                layout: EitherRefLayout::ToRef,
            },
        };
    }

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
    fn test_jetton_wallet_transfer_parser() -> Result<(), TonMessageError> {
        let result_jetton_transfer_msg = JettonWalletMessage::from_boc_hex(JETTON_TRANSFER_MSG)?;

        let transfer_message_cell = Arc::new(Cell::new(
            hex::decode(TRANSFER_PAYLOAD).unwrap(),
            862,
            vec![],
            false,
        )?);

        let expected_jetton_transfer_msg =
            JettonWalletMessage::JettonTransfer(JettonTransferMessage {
                query_id: 8819263745311958,
                amount: BigUint::from(1000000000u64),
                destination: TonAddress::from_str(
                    "EQB3ncyBUTjZUA5EnFKR5_EnOMI9V1tTEAAPaiU71gc4TiUt",
                )
                .unwrap()
                .to_msg_address(),
                response_destination: TonAddress::from_str(
                    "EQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM9c",
                )
                .unwrap()
                .to_msg_address(),
                custom_payload: None,
                forward_ton_amount: BigUint::from(215000000u64),
                forward_payload: EitherRef {
                    value: transfer_message_cell,
                    layout: EitherRefLayout::ToRef,
                },
            });

        assert_eq!(expected_jetton_transfer_msg, result_jetton_transfer_msg);
        Ok(())
    }

    #[test]
    fn test_jetton_internal_wallet_transfer_parser() -> Result<(), TonMessageError> {
        let result_jetton_internal_transfer_msg =
            JettonWalletMessage::from_boc_hex(JETTON_INTERNAL_TRANSFER_MSG)?;

        let transfer_message_cell = Arc::new(Cell::new(
            hex::decode(INTERNAL_TRANSFER_PAYLOAD).unwrap(),
            611,
            vec![],
            false,
        )?);

        let expected_jetton_internal_transfer_msg =
            JettonWalletMessage::JettonInternalTransfer(JettonInternalTransferMessage {
                query_id: 352352856990,
                amount: BigUint::from(1089377168u64),
                from_address: TonAddress::from_str(
                    "UQDzd5FDImOt9dBGRccVb7F487B0JDDa4ZgT-CGfDIQriSB-",
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
                    value: transfer_message_cell,
                    layout: EitherRefLayout::ToRef,
                },
            });

        assert_eq!(
            expected_jetton_internal_transfer_msg,
            result_jetton_internal_transfer_msg
        );
        Ok(())
    }
}
