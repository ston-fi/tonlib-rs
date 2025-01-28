//! Constants from soulbound nft standard
//! https://github.com/ton-blockchain/TEPs/blob/master/text/0085-sbt-standard.md

/// prove_ownership#04ded148
///   query_id:uint64
///   dest:MsgAddress
///   forward_payload:^Cell
///   with_content:Bool
/// = InternalMsgBody;
pub const SBT_PROVE_OWNERSHIP: u32 = 0x04ded148;

/// ownership_proof#0524c7ae
///   query_id:uint64
///   item_id:uint256
///   owner:MsgAddress
///   data:^Cell
///   revoked_at:uint64
///   content:(Maybe ^Cell)
/// = InternalMsgBody;
pub const SBT_OWNERSHIP_PROOF: u32 = 0x0524c7ae;

/// request_owner#d0c3bfea
///   query_id:uint64
///   dest:MsgAddress
///   forward_payload:^Cell
///   with_content:Bool
/// = InternalMsgBody;
pub const SBT_REQUEST_OWNER: u32 = 0xd0c3bfea;

/// owner_info#0dd607e3
///   query_id:uint64
///   item_id:uint256
///   initiator:MsgAddress
///   owner:MsgAddress  
///   data:^Cell
///   revoked_at:uint64
///   content:(Maybe ^Cell)
/// = InternalMsgBody;
pub const SBT_OWNER_INFO: u32 = 0x0dd607e3;

/// destroy#1f04537a
///   query_id:uint64
/// = InternalMsgBody;
pub const SBT_DESTROY: u32 = 0x1f04537a;

/// revoke#6f89f5e3
///    query_id:uint64
/// = InternalMsgBody;
pub const SBT_REVOKE: u32 = 0x1f04537a;

mod destroy;
mod owner_info;
mod ownership_proof;
mod prove_ownersip;
mod request_owner;
mod revoke;

pub use destroy::*;
pub use owner_info::*;
pub use ownership_proof::*;
pub use prove_ownersip::*;
pub use request_owner::*;
pub use revoke::*;
