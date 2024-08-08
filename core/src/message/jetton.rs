// Constants from jetton standard
// https://github.com/ton-blockchain/TEPs/blob/master/text/0074-jettons-standard.md

// crc32('transfer query_id:uint64 amount:VarUInteger 16 destination:MsgAddress response_destination:MsgAddress custom_payload:Maybe ^Cell forward_ton_amount:VarUInteger 16 forward_payload:Either Cell ^Cell = InternalMsgBody') = 0x8f8a7ea5 & 0x7fffffff = 0xf8a7ea5
// crc32('transfer_notification query_id:uint64 amount:VarUInteger 16 sender:MsgAddress forward_payload:Either Cell ^Cell = InternalMsgBody') = 0xf362d09c & 0x7fffffff = 0x7362d09c
// crc32('excesses query_id:uint64 = InternalMsgBody') = 0x553276db | 0x80000000 = 0xd53276db
// crc32('burn query_id:uint64 amount:VarUInteger 16 response_destination:MsgAddress custom_payload:Maybe ^Cell = InternalMsgBody') = 0x595f07bc & 0x7fffffff = 0x595f07bc
// crc32('internal_transfer query_id:uint64 amount:VarUInteger 16 from:MsgAddress response_address:MsgAddress forward_ton_amount:VarUInteger 16 forward_payload:Either Cell ^Cell = InternalMsgBody') = 0x978d4519 & 0x7fffffff = 0x178d4519
// crc32('burn_notification query_id:uint64 amount:VarUInteger 16 sender:MsgAddress response_destination:MsgAddress = InternalMsgBody') = 0x7bdd97de & 0x7fffffff = 0x7bdd97de

pub const JETTON_TRANSFER: u32 = 0x0f8a7ea5;
pub const JETTON_TRANSFER_NOTIFICATION: u32 = 0x7362d09c;
pub const JETTON_INTERNAL_TRANSFER: u32 = 0x178d4519;
pub const JETTON_EXCESSES: u32 = 0xd53276db;
pub const JETTON_BURN: u32 = 0x595f07bc;
pub const JETTON_BURN_NOTIFICATION: u32 = 0x7bdd97de;

mod burn;
mod transfer;
mod transfer_notification;

pub use burn::*;
pub use transfer::*;
pub use transfer_notification::*;
