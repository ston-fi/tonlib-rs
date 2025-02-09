use std::str::FromStr;

use crate::tlb_types::block::msg_address::{MsgAddrIntStd, MsgAddrIntVar, MsgAddress};
use crate::{TonAddress, TonAddressParseError};

impl FromStr for TonAddress {
    type Err = TonAddressParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 48 {
            // Some form of base64 address, check which one
            if s.contains('-') || s.contains('_') {
                TonAddress::from_base64_url(s)
            } else {
                TonAddress::from_base64_std(s)
            }
        } else {
            TonAddress::from_hex_str(s)
        }
    }
}

impl TryFrom<String> for TonAddress {
    type Error = TonAddressParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(value.as_str())
    }
}

impl TryFrom<MsgAddress> for TonAddress {
    type Error = TonAddressParseError;

    fn try_from(value: MsgAddress) -> Result<Self, Self::Error> {
        match value {
            MsgAddress::None(_) => Ok(TonAddress::null().clone()),
            MsgAddress::Ext(ext) => Err(TonAddressParseError::new(
                format!("{ext:?}"),
                "Can't load TonAddress from MsgAddressExt",
            )),
            MsgAddress::IntStd(addr) => TonAddress::try_from(addr),
            MsgAddress::IntVar(addr) => TonAddress::try_from(addr),
        }
    }
}

impl TryFrom<MsgAddrIntStd> for TonAddress {
    type Error = TonAddressParseError;

    fn try_from(value: MsgAddrIntStd) -> Result<Self, Self::Error> {
        TonAddress::from_tlb_data(value.workchain, value.address, 256, value.anycast.as_ref())
    }
}

impl TryFrom<MsgAddrIntVar> for TonAddress {
    type Error = TonAddressParseError;

    fn try_from(value: MsgAddrIntVar) -> Result<Self, Self::Error> {
        TonAddress::from_tlb_data(
            value.workchain,
            value.address,
            value.address_bit_len,
            value.anycast.as_ref(),
        )
    }
}
