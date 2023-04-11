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
    // tonlib_api.tl, line 230
    Init {
        options: Options,
    },
    //tonlib_api.tl, line 258
    #[serde(rename = "raw.getAccountState")]
    RawGetAccountState {
        account_address: AccountAddress,
    },
    // tonlib_api.tl, line 259
    #[serde(rename = "raw.getTransactions")]
    RawGetTransactions {
        account_address: AccountAddress,
        from_transaction_id: InternalTransactionId,
    },
    // tonlib_api.tl, line 260
    #[serde(rename = "raw.getTransactionsV2")]
    RawGetTransactionsV2 {
        account_address: AccountAddress,
        from_transaction_id: InternalTransactionId,
        count: u32,
        try_decode_messages: bool,
    },
    // tonlib_api.tl, line 261
    #[serde(rename = "raw.sendMessage")]
    RawSendMessage {
        #[serde(with = "Base64Standard")]
        body: Vec<u8>,
    },
    // tonlib_api.tl, line 262
    #[serde(rename = "raw.sendMessageReturnHash")]
    RawSendMessageReturnHash {
        #[serde(with = "Base64Standard")]
        body: Vec<u8>,
    },
    // tonlib_api.tl, line 266
    #[serde(rename = "sync")]
    Sync {},
    // tonlib_api.tl, line 279
    #[serde(rename = "getAccountState")]
    GetAccountState {
        account_address: AccountAddress,
    },
    // tonlib_api.tl, line 293
    #[serde(rename = "smc.load")]
    SmcLoad {
        account_address: AccountAddress,
    },
    // tonlib_api.tl, line 298
    #[serde(rename = "smc.runGetMethod")]
    SmcRunGetMethod {
        id: i64,
        method: SmcMethodId,
        stack: Vec<TvmStackEntry>,
    },
    // tonlib_api.tl, line 311
    #[serde(rename = "blocks.getMasterchainInfo")]
    BlocksGetMasterchainInfo {},
    // tonlib_api.tl, line 312
    #[serde(rename = "blocks.getShards")]
    BlocksGetShards {
        id: BlockIdExt,
    },
    // tonlib_api.tl, line 313
    #[serde(rename = "blocks.lookupBlock")]
    BlocksLookupBlock {
        mode: i32,
        id: BlockId,
        lt: i64,
        utime: i32,
    },
    // tonlib_api.tl, line 314
    #[serde(rename = "blocks.getTransactions")]
    BlocksGetTransactions {
        id: BlockIdExt,
        mode: u32,
        count: u32,
        after: BlocksAccountTransactionId,
    },
    // tonlib_api.tl, line 314
    #[serde(rename = "blocks.getBlockHeader")]
    GetBlockHeader {
        id: BlockIdExt,
    },
    // tonlib_api.tl, line 336
    SetLogVerbosityLevel {
        new_verbosity_level: u32,
    },
    // tonlib_api.tl, line 339
    GetLogVerbosityLevel {},
}
