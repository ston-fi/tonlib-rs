use num_bigint::BigUint;

use super::ZERO_COINS;
use crate::TonAddress;

#[derive(Clone, Debug, PartialEq)]
pub enum CommonMsgInfo {
    InternalMessage(InternalMessage),
    ExternalIncomingMessage(ExternalIncomingMessage),
    ExternalOutgoingMessage(ExternalOutgoingMessage),
}

#[derive(Clone, Debug, PartialEq)]
pub struct InternalMessage {
    /// Hyper cube routing flag.
    pub ihr_disabled: bool,
    /// Message should be bounced if there are errors during processing.
    /// If message's flat bounce = 1, it calls bounceable.
    pub bounce: bool,
    /// Flag that describes, that message itself is a result of bounce.
    pub bounced: bool,
    /// Address of smart contract sender of message.
    pub src: TonAddress,
    /// Address of smart contract destination of message.
    pub dest: TonAddress,
    /// Structure which describes currency information including total funds transferred in message.
    pub value: BigUint,
    /// Fees for hyper routing delivery
    pub ihr_fee: BigUint,
    /// Fees for forwarding messages assigned by validators
    pub fwd_fee: BigUint,
    /// Logic time of sending message assigned by validator. Using for odering actions in smart contract.
    pub created_lt: u64,
    /// Unix time
    pub created_at: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExternalIncomingMessage {
    /// Address of a external sender of the message.
    pub src: TonAddress,
    /// Address of smart contract destination of message.
    pub dest: TonAddress,
    /// Fee for executing and delivering of message.
    pub import_fee: BigUint,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExternalOutgoingMessage {
    /// Address of a external sender of the message.
    pub src: TonAddress,
    /// Address of smart contract destination of message.
    pub dest: TonAddress,
    /// Logic time of sending message assigned by validator. Using for odering actions in smart contract.
    pub created_lt: u64,
    /// Unix time
    pub created_at: u32,
}

impl CommonMsgInfo {
    pub fn new_default_internal(dest: &TonAddress, value: &BigUint) -> Self {
        CommonMsgInfo::InternalMessage(InternalMessage {
            ihr_disabled: false,
            bounce: true,
            bounced: true,
            src: TonAddress::null().clone(),
            dest: dest.clone(),
            value: value.clone(),
            ihr_fee: ZERO_COINS.clone(),
            fwd_fee: ZERO_COINS.clone(),
            created_lt: 0,
            created_at: 0,
        })
    }
    pub fn src(&self) -> TonAddress {
        match self {
            CommonMsgInfo::InternalMessage(m) => m.src.clone(),
            CommonMsgInfo::ExternalIncomingMessage(m) => m.src.clone(),
            CommonMsgInfo::ExternalOutgoingMessage(m) => m.src.clone(),
        }
    }
    pub fn dest(&self) -> TonAddress {
        match self {
            CommonMsgInfo::InternalMessage(m) => m.dest.clone(),
            CommonMsgInfo::ExternalIncomingMessage(m) => m.dest.clone(),
            CommonMsgInfo::ExternalOutgoingMessage(m) => m.dest.clone(),
        }
    }

    // todo impl others and think about better api
}
