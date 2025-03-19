use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

use base64::engine::general_purpose::{STANDARD_NO_PAD, URL_SAFE_NO_PAD};
use base64::Engine;
use crc::Crc;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{TonAddressParseError, TonHash, ZERO_HASH};
use crate::cell::{rewrite_bits, ArcCell, CellBuilder, TonCellError};
use crate::tlb_types::block::msg_address::{
    Anycast, MsgAddrIntStd, MsgAddrIntVar, MsgAddress, MsgAddressInt,
};
use crate::tlb_types::block::state_init::StateInit;
use crate::tlb_types::traits::TLBObject;

const CRC_16_XMODEM: Crc<u16> = Crc::<u16>::new(&crc::CRC_16_XMODEM);

#[derive(PartialEq, Eq, Clone, Hash)]
pub struct TonAddress {
    pub workchain: i32,
    pub hash_part: TonHash,
}

impl TonAddress {
    pub const NULL: TonAddress = TonAddress {
        workchain: 0,
        hash_part: ZERO_HASH,
    };

    pub const fn new(workchain: i32, hash_part: TonHash) -> TonAddress {
        TonAddress {
            workchain,
            hash_part,
        }
    }

    pub fn derive(
        workchain: i32,
        code: ArcCell,
        data: ArcCell,
    ) -> Result<TonAddress, TonCellError> {
        let state_init = StateInit::new(code, data);
        Ok(TonAddress::new(workchain, state_init.cell_hash()?))
    }

    pub fn from_msg_address<T: Into<MsgAddress>>(
        addr: T,
    ) -> Result<TonAddress, TonAddressParseError> {
        match addr.into() {
            MsgAddress::None(_) => Ok(TonAddress::NULL),
            MsgAddress::Ext(ext) => Err(TonAddressParseError::new(
                format!("{ext:?}"),
                "Can't load TonAddress from MsgAddressExt",
            )),
            MsgAddress::IntStd(addr) => {
                TonAddress::from_tlb_data(addr.workchain, addr.address, 256, addr.anycast.as_ref())
            }
            MsgAddress::IntVar(addr) => {
                TonAddress::from_tlb_data(addr.workchain, addr.address, 256, addr.anycast.as_ref())
            }
        }
    }

    pub fn from_hex_str(s: &str) -> Result<TonAddress, TonAddressParseError> {
        let parts: Vec<&str> = s.split(':').collect();

        if parts.len() != 2 {
            return Err(TonAddressParseError::new(
                s,
                "Invalid hex address string: wrong address format",
            ));
        }

        let maybe_wc = parts[0].parse::<i32>();
        let wc = match maybe_wc {
            Ok(wc) => wc,
            Err(_) => {
                return Err(TonAddressParseError::new(
                    s,
                    "Invalid hex address string: parse int error",
                ))
            }
        };

        let maybe_decoded_hash_part = hex::decode(parts[1]);
        let decoded_hash_part = match maybe_decoded_hash_part {
            Ok(decoded_hash_part) => decoded_hash_part,
            Err(_) => {
                return Err(TonAddressParseError::new(
                    s,
                    "Invalid hex address string: base64 decode error",
                ))
            }
        };

        let maybe_hash_part = decoded_hash_part.as_slice().try_into();
        let hash_part = match maybe_hash_part {
            Ok(hash_part) => hash_part,
            Err(_) => {
                return Err(TonAddressParseError::new(
                    s,
                    "Invalid hex address string: unexpected error",
                ))
            }
        };

        let addr = TonAddress::new(wc, hash_part);
        Ok(addr)
    }

    pub fn from_base64_url(s: &str) -> Result<TonAddress, TonAddressParseError> {
        Ok(Self::from_base64_url_flags(s)?.0)
    }

    /// Parses url-safe base64 representation of an address
    ///
    /// # Returns
    /// the address, non-bounceable flag, non-production flag.
    pub fn from_base64_url_flags(
        s: &str,
    ) -> Result<(TonAddress, bool, bool), TonAddressParseError> {
        if s.len() != 48 {
            return Err(TonAddressParseError::new(
                s,
                "Invalid base64url address: Wrong length",
            ));
        }
        let maybe_bytes = URL_SAFE_NO_PAD.decode(s);
        let bytes = match maybe_bytes {
            Ok(bytes) => bytes,
            Err(_) => {
                return Err(TonAddressParseError::new(
                    s,
                    "Invalid base64url address: Base64 decode error",
                ))
            }
        };
        let maybe_slice = bytes.as_slice().try_into();
        let slice = match maybe_slice {
            Ok(slice) => slice,
            Err(_) => {
                return Err(TonAddressParseError::new(
                    s,
                    "Invalid base64url address: Unexpected error",
                ))
            }
        };

        Self::from_base64_src(slice, s)
    }

    pub fn from_base64_std(s: &str) -> Result<TonAddress, TonAddressParseError> {
        Ok(Self::from_base64_std_flags(s)?.0)
    }

    /// Parses standard base64 representation of an address
    ///
    /// # Returns
    /// the address, non-bounceable flag, non-production flag.
    pub fn from_base64_std_flags(
        s: &str,
    ) -> Result<(TonAddress, bool, bool), TonAddressParseError> {
        if s.len() != 48 {
            return Err(TonAddressParseError::new(
                s,
                "Invalid base64std address: Invalid length",
            ));
        }

        let maybe_vec = STANDARD_NO_PAD.decode(s);
        let vec = match maybe_vec {
            Ok(bytes) => bytes,
            Err(_) => {
                return Err(TonAddressParseError::new(
                    s,
                    "Invalid base64std address: Base64 decode error",
                ))
            }
        };
        let maybe_bytes = vec.as_slice().try_into();
        let bytes = match maybe_bytes {
            Ok(b) => b,
            Err(_) => {
                return Err(TonAddressParseError::new(
                    s,
                    "Invalid base64std: Unexpected error",
                ))
            }
        };

        Self::from_base64_src(bytes, s)
    }

    /// Parses decoded base64 representation of an address
    ///
    /// # Returns
    /// the address, non-bounceable flag, non-production flag.
    fn from_base64_src(
        bytes: &[u8; 36],
        src: &str,
    ) -> Result<(TonAddress, bool, bool), TonAddressParseError> {
        let (testnet, non_bounceable) = match bytes[0] {
            0x11 => (false, false),
            0x51 => (false, true),
            0x91 => (true, false),
            0xD1 => (true, true),
            _ => {
                return Err(TonAddressParseError::new(
                    src,
                    "Invalid base64src address: Wrong tag byte",
                ))
            }
        };
        let workchain = bytes[1] as i8 as i32;
        let calc_crc = CRC_16_XMODEM.checksum(&bytes[0..34]);
        let addr_crc = ((bytes[34] as u16) << 8) | bytes[35] as u16;
        if calc_crc != addr_crc {
            return Err(TonAddressParseError::new(
                src,
                "Invalid base64src address: CRC mismatch",
            ));
        }
        let hash_part = TonHash::try_from(&bytes[2..34])?;
        let addr = TonAddress {
            workchain,
            hash_part,
        };
        Ok((addr, non_bounceable, testnet))
    }

    pub fn from_tlb_data(
        workchain: i32,
        mut address: Vec<u8>,
        address_bit_len: u16,
        maybe_anycast: Option<&Anycast>,
    ) -> Result<TonAddress, TonAddressParseError> {
        let anycast = match maybe_anycast {
            Some(anycast) => anycast,
            None => {
                let hash = TonHash::try_from(address.as_slice())?;
                return Ok(TonAddress::new(workchain, hash));
            }
        };

        if address_bit_len < anycast.depth.into() {
            let err_msg = format!(
                "rewrite_pfx has {} bits, but address has only {address_bit_len} bits",
                anycast.depth
            );
            let ext_addr_str = format!("address: {:?}, anycast: {:?}", address, anycast);
            return Err(TonAddressParseError::new(ext_addr_str, err_msg));
        }

        let new_prefix = anycast.rewrite_pfx.as_slice();

        let bits = anycast.depth as usize;
        if !rewrite_bits(new_prefix, 0, address.as_mut_slice(), 0, bits) {
            let err_msg = format!("Failed to rewrite address prefix with new_prefix={new_prefix:?}, address={address:?}, bits={bits}");
            let ext_addr_str = format!("address: {:?}, anycast: {:?}", address, anycast);
            return Err(TonAddressParseError::new(ext_addr_str, err_msg));
        }

        Ok(TonAddress::new(workchain, TonHash::try_from(address)?))
    }

    pub fn to_hex(&self) -> String {
        format!("{}:{}", self.workchain, self.hash_part.to_hex())
    }

    pub fn to_base64_url(&self) -> String {
        self.to_base64_url_flags(false, false)
    }

    pub fn to_base64_url_flags(&self, non_bounceable: bool, testnet: bool) -> String {
        let mut buf: [u8; 36] = [0; 36];
        self.to_base64_src(&mut buf, non_bounceable, testnet);
        URL_SAFE_NO_PAD.encode(buf)
    }

    pub fn to_base64_std(&self) -> String {
        self.to_base64_std_flags(false, false)
    }

    pub fn to_base64_std_flags(&self, non_bounceable: bool, testnet: bool) -> String {
        let mut buf: [u8; 36] = [0; 36];
        self.to_base64_src(&mut buf, non_bounceable, testnet);
        STANDARD_NO_PAD.encode(buf)
    }

    fn to_base64_src(&self, bytes: &mut [u8; 36], non_bounceable: bool, testnet: bool) {
        let tag: u8 = match (testnet, non_bounceable) {
            (false, false) => 0x11,
            (false, true) => 0x51,
            (true, false) => 0x91,
            (true, true) => 0xD1,
        };
        bytes[0] = tag;
        bytes[1] = (self.workchain & 0xff) as u8;
        bytes[2..34].clone_from_slice(self.hash_part.as_slice());
        let crc = CRC_16_XMODEM.checksum(&bytes[0..34]);
        bytes[34] = ((crc >> 8) & 0xff) as u8;
        bytes[35] = (crc & 0xff) as u8;
    }

    pub fn to_msg_address(&self) -> MsgAddress {
        if self == &TonAddress::NULL {
            return MsgAddress::NONE;
        }
        MsgAddress::IntStd(MsgAddrIntStd {
            anycast: None,
            workchain: self.workchain,
            address: self.hash_part.to_vec(),
        })
    }
}

impl PartialOrd for TonAddress {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let self_cell_hash = CellBuilder::new()
            .store_address(self)
            .and_then(|builder| builder.build())
            .map(|cell| cell.cell_hash())
            .ok();

        let other_cell_hash = CellBuilder::new()
            .store_address(other)
            .and_then(|builder| builder.build())
            .map(|cell| cell.cell_hash())
            .ok();

        match (self_cell_hash, other_cell_hash) {
            (Some(hash0), Some(hash1)) => Some(hash0.cmp(&hash1)),
            _ => None,
        }
    }
}

impl Display for TonAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_base64_url().as_str())
    }
}

impl Debug for TonAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.to_base64_url().as_str())
    }
}

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
            MsgAddress::None(_) => Ok(TonAddress::NULL),
            MsgAddress::Ext(ext) => Err(TonAddressParseError::new(
                format!("{ext:?}"),
                "Can't load TonAddress from MsgAddressExt",
            )),
            MsgAddress::IntStd(addr) => TonAddress::try_from(addr),
            MsgAddress::IntVar(addr) => TonAddress::try_from(addr),
        }
    }
}

impl TryFrom<MsgAddressInt> for TonAddress {
    type Error = TonAddressParseError;

    fn try_from(value: MsgAddressInt) -> Result<Self, Self::Error> {
        match value {
            MsgAddressInt::Std(addr) => TonAddress::try_from(addr),
            MsgAddressInt::Var(addr) => TonAddress::try_from(addr),
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

impl Serialize for TonAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_base64_url().as_str())
    }
}

struct TonAddressVisitor;

impl Visitor<'_> for TonAddressVisitor {
    type Value = TonAddress;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("an string representing TON address in Hex or Base64 format")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        v.parse().map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for TonAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(TonAddressVisitor)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::Arc;

    use num_bigint::BigUint;
    use num_traits::Zero;
    use serde_json::Value;

    use super::TonAddressParseError;
    use crate::cell::{BagOfCells, Cell, CellBuilder};
    use crate::tlb_types::block::msg_address::{MsgAddrIntStd, MsgAddress};
    use crate::tlb_types::traits::TLBObject;
    use crate::{TonAddress, TonHash};

    #[test]
    fn format_works() -> anyhow::Result<()> {
        let bytes =
            TonHash::from_hex("e4d954ef9f4e1250a26b5bbad76a1cdd17cfd08babad6f4c23e372270aef6f76")?;
        let addr = TonAddress::new(0, bytes);
        assert_eq!(
            addr.to_hex(),
            "0:e4d954ef9f4e1250a26b5bbad76a1cdd17cfd08babad6f4c23e372270aef6f76"
        );
        assert_eq!(
            addr.to_base64_url(),
            "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR"
        );
        assert_eq!(
            addr.to_base64_std(),
            "EQDk2VTvn04SUKJrW7rXahzdF8/Qi6utb0wj43InCu9vdjrR"
        );
        Ok(())
    }

    #[test]
    fn parse_format_works() -> anyhow::Result<()> {
        let bytes =
            TonHash::from_hex("e4d954ef9f4e1250a26b5bbad76a1cdd17cfd08babad6f4c23e372270aef6f76")?;
        let addr = TonAddress::new(0, bytes);
        assert_eq!(
            TonAddress::from_hex_str(
                "0:e4d954ef9f4e1250a26b5bbad76a1cdd17cfd08babad6f4c23e372270aef6f76"
            )?,
            addr
        );
        assert_eq!(
            TonAddress::from_base64_url("EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR")?,
            addr
        );
        assert_eq!(
            TonAddress::from_base64_std("EQDk2VTvn04SUKJrW7rXahzdF8/Qi6utb0wj43InCu9vdjrR")?,
            addr
        );
        Ok(())
    }

    #[test]
    fn parse_works() -> anyhow::Result<()> {
        let bytes =
            TonHash::from_hex("e4d954ef9f4e1250a26b5bbad76a1cdd17cfd08babad6f4c23e372270aef6f76")?;
        let addr = TonAddress::new(0, bytes);
        assert_eq!(
            "0:e4d954ef9f4e1250a26b5bbad76a1cdd17cfd08babad6f4c23e372270aef6f76"
                .parse::<TonAddress>()?,
            addr
        );
        assert_eq!(
            "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR".parse::<TonAddress>()?,
            addr
        );
        assert_eq!(
            "EQDk2VTvn04SUKJrW7rXahzdF8/Qi6utb0wj43InCu9vdjrR".parse::<TonAddress>()?,
            addr
        );
        Ok(())
    }

    #[test]
    fn test_derive() -> anyhow::Result<()> {
        let user_addr = TonAddress::from_str("UQAO9JsDEbOjnb8AZRyxNHiODjVeAvgR2n03T0utYgkpx-K0")?;
        let pool_addr = TonAddress::from_str("EQDMk-2P8ziShAYGcnYq-z_U33zA_Ynt88iav4PwkSGRru2B")?;
        let code_cell = BagOfCells::parse_hex("b5ee9c7201010201002d00010eff0088d0ed1ed801084202e70a306c00272796243f569ce0c928ea4cfc9f1b65c5b0066e382159f5e80df5")?.single_root()?;
        let data_cell = CellBuilder::new()
            .store_address(&user_addr)?
            .store_address(&pool_addr)?
            .store_coins(&BigUint::zero())?
            .store_coins(&BigUint::zero())?
            .build()?;
        let derived_addr = TonAddress::derive(0, code_cell, Arc::new(data_cell))?;

        let expected_addr =
            TonAddress::from_str("EQBWxdw3leOoaHqcK3ATf0T7ae5M8XS6jiP_Din4mh7o7gj2")?;
        assert_eq!(derived_addr, expected_addr);
        Ok(())
    }

    #[test]
    fn test_derive_stonfi_pool() -> anyhow::Result<()> {
        let code_cell = Cell::from_boc_hex("b5ee9c7201010101002300084202a9338ecd624ca15d37e4a8d9bf677ddc9b84f0e98f05f2fb84c7afe332a281b4")?;
        let data_cell = Cell::from_boc_hex("b5ee9c720101040100b900010d000000000000050102c9801459f7c0a12bb4ac4b78a788c425ee4d52f8b6041dda17b77b09fc5a03e894d6900287cd9fbe2ea663415da0aa6bbdf0cb136abe9c4f45214dd259354b80da8c265a006aebb27f5d0f1daf43e200f52408f3eb9ff5610f5b43284224644e7c6a590d14400203084202c00836440d084e44fb94316132ac5a21417ef4f429ee09b5560b5678b334c3e8084202c95a2ed22ab516f77f9d4898dc4578e72f18a2448e8f6832334b0b4bf501bc79")?;
        let address = TonAddress::derive(0, code_cell.to_arc(), data_cell.to_arc())?;
        let exp_addr = TonAddress::from_str("EQAdltEfzXG_xteLFaKFGd-HPVKrEJqv_FdC7z2roOddRNdM")?;
        assert_eq!(address, exp_addr);
        // assert!(false);
        Ok(())
    }

    #[test]
    fn try_from_works() -> anyhow::Result<()> {
        let bytes =
            TonHash::from_hex("e4d954ef9f4e1250a26b5bbad76a1cdd17cfd08babad6f4c23e372270aef6f76")?;
        let addr = TonAddress::new(0, bytes);
        let res: TonAddress = "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR"
            .to_string()
            .try_into()?;
        assert_eq!(res, addr);
        Ok(())
    }

    #[test]
    fn parse_verifies_crc() -> Result<(), TonAddressParseError> {
        let res = "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjra".parse::<TonAddress>();
        assert!(res.is_err());
        Ok(())
    }

    #[test]
    fn serialization_works() -> anyhow::Result<()> {
        let expected = "\"EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR\"";

        let res = "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR".parse::<TonAddress>()?;
        let serial = serde_json::to_string(&res)?;
        println!("{}", serial);
        assert_eq!(serial.as_str(), expected);

        let res = "0:e4d954ef9f4e1250a26b5bbad76a1cdd17cfd08babad6f4c23e372270aef6f76"
            .parse::<TonAddress>()?;
        let serial = serde_json::to_string(&res)?;
        println!("{}", serial);
        assert_eq!(serial.as_str(), expected);

        let res = "EQDk2VTvn04SUKJrW7rXahzdF8/Qi6utb0wj43InCu9vdjrR".parse::<TonAddress>()?;
        let serial = serde_json::to_string(&res)?;
        println!("{}", serial);
        assert_eq!(serial.as_str(), expected);

        Ok(())
    }

    #[test]
    fn deserialization_works() -> anyhow::Result<()> {
        let address = "EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR";
        let a = format!("\"{}\"", address);
        let deserial: TonAddress = serde_json::from_str(a.as_str())?;
        let expected = address.parse()?;
        println!("{}", deserial);
        assert_eq!(deserial, expected);

        let address = "EQDk2VTvn04SUKJrW7rXahzdF8/Qi6utb0wj43InCu9vdjrR";
        let a = format!("\"{}\"", address);
        let deserial: TonAddress = serde_json::from_str(a.as_str())?;
        let expected = address.parse()?;
        println!("{}", deserial);
        assert_eq!(deserial, expected);

        let address = "0:e4d954ef9f4e1250a26b5bbad76a1cdd17cfd08babad6f4c23e372270aef6f76";
        let a = format!("\"{}\"", address);
        let deserial: TonAddress = serde_json::from_str(a.as_str())?;
        let expected = address.parse()?;
        println!("{}", deserial);
        assert_eq!(deserial, expected);

        let address =
            String::from("0:e4d954ef9f4e1250a26b5bbad76a1cdd17cfd08babad6f4c23e372270aef6f76");
        let deserial: TonAddress = serde_json::from_value(Value::String(address.clone()))?;
        let expected = address.clone().parse()?;
        println!("{}", deserial);
        assert_eq!(deserial, expected);

        let address = "124";
        let a = format!("\"{}\"", address);
        let deserial: serde_json::Result<TonAddress> = serde_json::from_str(a.as_str());
        assert!(deserial.is_err());

        Ok(())
    }

    #[test]
    fn ordering_works() -> Result<(), TonAddressParseError> {
        let address0 = TonAddress::from_str("EQBKwtMZSZurMxGp7FLZ_lM9t54_ECEsS46NLR3qfIwwTnKW")?;
        let address1 = TonAddress::from_str("EQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAM9c")?;

        let cmp_result = address0 < address1;
        assert!(cmp_result);
        Ok(())
    }

    #[test]
    fn test_to_msg_addr_std() -> anyhow::Result<()> {
        let address = TonAddress::from_str("EQDk2VTvn04SUKJrW7rXahzdF8_Qi6utb0wj43InCu9vdjrR")?;
        let msg_addr = address.to_msg_address();
        let expected = MsgAddress::IntStd(MsgAddrIntStd {
            anycast: None,
            workchain: 0,
            address: hex::decode(
                "e4d954ef9f4e1250a26b5bbad76a1cdd17cfd08babad6f4c23e372270aef6f76",
            )?,
        });
        assert_eq!(msg_addr, expected);
        Ok(())
    }
}
