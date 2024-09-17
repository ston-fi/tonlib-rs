/// Constants from jetton standard
/// https://github.com/ton-blockchain/TEPs/blob/master/text/0062-jettons-standard.md

/// transfer#5fcc3d14
///   query_id:uint64
///   new_owner:MsgAddress
///   response_destination:MsgAddress
///   custom_payload:(Maybe ^Cell)
///   forward_amount:(VarUInteger 16)
///   forward_payload:(Either Cell ^Cell)
/// = InternalMsgBody;
pub const NFT_TRANSFER: u32 = 0x5fcc3d14;

/// ownership_assigned#0x05138d91
///   query_id:uint64
///   prev_owner:MsgAddress
///   forward_payload:(Either Cell ^Cell)
/// = InternalMsgBody;
pub const NFT_OWNERSHIP_ASSIGNED: u32 = 0x05138d91;

/// get_static_data#2fcb26a2
///   query_id:uint64
/// = InternalMsgBody;
pub const NFT_GET_STATIC_DATA: u32 = 0x2fcb26a2;

/// report_static_data#0x8b771735
///   query_id:uint64
///   index:uint256
///   collection:MsgAddress
/// = InternalMsgBody
pub const NFT_REPORT_STATIC_DATA: u32 = 0x8b771735;

mod get_static_data;
mod ownership_assigned;
mod report_static_data;
mod transfer;

pub use get_static_data::*;
pub use ownership_assigned::*;
pub use report_static_data::*;
pub use transfer::*;
