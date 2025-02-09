use std::fmt::Formatter;

use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::TonAddress;

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
