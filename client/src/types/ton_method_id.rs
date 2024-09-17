use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};

use crc::{Crc, CRC_32_ISO_HDLC};
use lazy_static::lazy_static;

use crate::tl::SmcMethodId;

lazy_static! {
    pub static ref CRC_16_XMODEM: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_XMODEM);
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub enum TonMethodId {
    Number(i32),
    Name(Cow<'static, str>),
}

impl TonMethodId {
    pub fn from_prototype(prototype: &str) -> TonMethodId {
        let opcode = calc_opcode(prototype);
        Self::Number(opcode)
    }
}

impl From<&'static str> for TonMethodId {
    fn from(value: &'static str) -> Self {
        TonMethodId::Name(Cow::Borrowed(value))
    }
}

impl From<String> for TonMethodId {
    fn from(value: String) -> Self {
        TonMethodId::Name(Cow::Owned(value))
    }
}

impl From<i32> for TonMethodId {
    fn from(value: i32) -> Self {
        TonMethodId::Number(value)
    }
}

impl From<&TonMethodId> for SmcMethodId {
    fn from(value: &TonMethodId) -> Self {
        match value {
            TonMethodId::Number(v) => SmcMethodId::Number { number: *v },
            TonMethodId::Name(v) => SmcMethodId::Name { name: v.clone() },
        }
    }
}

impl TonMethodId {
    pub fn to_id(&self) -> i32 {
        match self {
            TonMethodId::Name(name) => CRC_16_XMODEM.checksum(name.as_bytes()) as i32 | 0x10000,
            TonMethodId::Number(id) => *id,
        }
    }
}

impl Display for TonMethodId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TonMethodId::Number(n) => write!(f, "#{:08x}", n),
            TonMethodId::Name(m) => write!(f, "'{}'", m),
        }
    }
}

impl Debug for TonMethodId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

fn calc_checksum(command: &str) -> u32 {
    let crc = Crc::<u32>::new(&CRC_32_ISO_HDLC);
    crc.checksum(command.as_bytes())
}

fn calc_opcode(command: &str) -> i32 {
    (calc_checksum(command) & 0x7fffffff) as i32
}

#[cfg(test)]
mod tests {
    use crate::types::TonMethodId;

    #[test]
    fn test_hex_format() -> anyhow::Result<()> {
        let method_id: TonMethodId = 0x1234beef.into();
        let s = format!("{}", method_id);
        assert_eq!(s, "#1234beef");
        Ok(())
    }

    #[test]
    fn test_opcode() -> anyhow::Result<()> {
        let p = "transfer query_id:uint64 amount:VarUInteger 16 destination:MsgAddress \
        response_destination:MsgAddress custom_payload:Maybe ^Cell forward_ton_amount:VarUInteger 16 \
        forward_payload:Either Cell ^Cell = InternalMsgBody";
        let method_id: TonMethodId = TonMethodId::from_prototype(p);
        assert_eq!(method_id, TonMethodId::Number(0x0f8a7ea5));
        Ok(())
    }
}
