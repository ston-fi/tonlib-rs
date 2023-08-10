use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;

use crate::tl::stack::TvmStackEntry;
use crate::tl::types::{
    AccountAddress, BlockId, BlockIdExt, BlocksAccountTransactionId, InternalTransactionId,
    Options, SmcMethodId,
};
use crate::tl::Base64Standard;

#[derive(IntoStaticStr, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(tag = "@type", rename_all = "camelCase")]
pub enum TonFunction {
    // tonlib_api.tl, line 210
    LiteServerInfo {
        now: i64,
        version: i32,
        capabilities: i64,
    },
    // tonlib_api.tl, line 232
    Init {
        options: Options,
    },
    //tonlib_api.tl, line 260
    #[serde(rename = "raw.getAccountState")]
    RawGetAccountState {
        account_address: AccountAddress,
    },
    // tonlib_api.tl, line 262
    #[serde(rename = "raw.getTransactions")]
    RawGetTransactions {
        account_address: AccountAddress,
        from_transaction_id: InternalTransactionId,
    },
    // tonlib_api.tl, line 263
    #[serde(rename = "raw.getTransactionsV2")]
    RawGetTransactionsV2 {
        account_address: AccountAddress,
        from_transaction_id: InternalTransactionId,
        count: u32,
        try_decode_messages: bool,
    },
    // tonlib_api.tl, line 264
    #[serde(rename = "raw.sendMessage")]
    RawSendMessage {
        #[serde(with = "Base64Standard")]
        body: Vec<u8>,
    },
    // tonlib_api.tl, line 265
    #[serde(rename = "raw.sendMessageReturnHash")]
    RawSendMessageReturnHash {
        #[serde(with = "Base64Standard")]
        body: Vec<u8>,
    },
    // tonlib_api.tl, line 269
    #[serde(rename = "sync")]
    Sync {},
    // tonlib_api.tl, line 282
    #[serde(rename = "getAccountState")]
    GetAccountState {
        account_address: AccountAddress,
    },
    // tonlib_api.tl, line 300
    #[serde(rename = "smc.load")]
    SmcLoad {
        account_address: AccountAddress,
    },
    // tonlib_api.tl, line 301
    #[serde(rename = "smc.loadByTransaction")]
    SmcLoadByTransaction {
        account_address: AccountAddress,
        transaction_id: InternalTransactionId,
    },
    // tonlib_api.tl, line 302
    #[serde(rename = "smc.forget")]
    SmcForget {
        id: i64,
    },
    // tonlib_api.tl, line 303
    #[serde(rename = "smc.getCode")]
    SmcGetCode {
        id: i64,
    },
    // tonlib_api.tl, line 304
    #[serde(rename = "smc.getData")]
    SmcGetData {
        id: i64,
    },
    // tonlib_api.tl, line 305
    #[serde(rename = "smc.getState")]
    SmcGetState {
        id: i64,
    },
    // tonlib_api.tl, line 306
    #[serde(rename = "smc.runGetMethod")]
    SmcRunGetMethod {
        id: i64,
        method: SmcMethodId,
        stack: Vec<TvmStackEntry>,
    },
    // tonlib_api.tl, line 319
    #[serde(rename = "blocks.getMasterchainInfo")]
    BlocksGetMasterchainInfo {},
    // tonlib_api.tl, line 320
    #[serde(rename = "blocks.getShards")]
    BlocksGetShards {
        id: BlockIdExt,
    },
    // tonlib_api.tl, line 321
    #[serde(rename = "blocks.lookupBlock")]
    BlocksLookupBlock {
        mode: i32,
        id: BlockId,
        lt: i64,
        utime: i32,
    },
    // tonlib_api.tl, line 288
    #[serde(rename = "getConfigParam")]
    GetConfigParam {
        mode: u32,
        param: u32,
    },
    // tonlib_api.tl, line 322
    #[serde(rename = "blocks.getTransactions")]
    BlocksGetTransactions {
        id: BlockIdExt,
        mode: u32,
        count: u32,
        after: BlocksAccountTransactionId,
    },
    // tonlib_ai.tl, line 335
    #[serde(rename = "liteServer.getInfo")]
    LiteServerGetInfo {},
    // tonlib_api.tl, line 324
    #[serde(rename = "blocks.getBlockHeader")]
    GetBlockHeader {
        id: BlockIdExt,
    },
    // tonlib_api.tl, line 345
    SetLogVerbosityLevel {
        new_verbosity_level: u32,
    },
    // tonlib_api.tl, line 348
    GetLogVerbosityLevel {},
}
