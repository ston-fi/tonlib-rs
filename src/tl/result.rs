use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;

use crate::tl::stack::TvmCell;
use crate::tl::types::{
    BlockIdExt, BlocksHeader, BlocksMasterchainInfo, BlocksShards, BlocksTransactions, ConfigInfo,
    FullAccountState, LiteServerInfo, LogVerbosityLevel, OptionsInfo, RawExtMessageInfo,
    RawFullAccountState, RawTransactions, SmcInfo, SmcRunResult, UpdateSyncState,
};

#[derive(IntoStaticStr, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(tag = "@type", rename_all = "camelCase")]
pub enum TonResult {
    // tonlib_api.tl, line 20
    Error {
        code: i32,
        message: String,
    },
    // tonlib_api.tl, line 21
    Ok {},
    // tonlib_api.tl, line 30
    #[serde(rename = "options.info")]
    OptionsInfo(OptionsInfo),
    // tonlib_api.tl, line 51
    #[serde(rename = "ton.blockIdExt")]
    BlockIdExt(BlockIdExt),
    // tonlib_api.tl, line 53
    #[serde(rename = "raw.fullAccountState")]
    RawFullAccountState(RawFullAccountState),
    // tonlib_api.tl, line 56
    #[serde(rename = "raw.transactions")]
    RawTransactions(RawTransactions),
    // tonlib_api.tl, line 58
    #[serde(rename = "raw.extMessageInfo")]
    RawExtMessageInfo(RawExtMessageInfo),
    // tonlib_api.tl, line 88
    #[serde(rename = "fullAccountState")]
    FullAccountState(FullAccountState),
    // tonlib_api.tl, line 177
    #[serde(rename = "smc.info")]
    SmcInfo(SmcInfo),
    // tonlib_api.tl, line 182
    #[serde(rename = "smc.runResult")]
    SmcRunResult(SmcRunResult),
    // tonlib_api.tl, line 188
    #[serde(rename = "updateSyncState")]
    UpdateSyncState(UpdateSyncState),
    // tonlib_api.tl, line 203
    #[serde(rename = "liteServer.info")]
    LiteServerInfo(LiteServerInfo),
    // tonlib_api.tl, line 210
    #[serde(rename = "logVerbosityLevel")]
    LogVerbosityLevel(LogVerbosityLevel),
    // tonlib_api.tl, line 213
    #[serde(rename = "blocks.masterchainInfo")]
    BlocksMasterchainInfo(BlocksMasterchainInfo),
    // tonlib_api.tl, line 214
    #[serde(rename = "blocks.shards")]
    BlocksShards(BlocksShards),
    // tonlib_api.tl, line 217
    #[serde(rename = "blocks.transactions")]
    BlocksTransactions(BlocksTransactions),
    // tonlib_api.tl, line 218
    #[serde(rename = "blocks.header")]
    BlocksHeader(BlocksHeader),
    // tonlib_api.tl, line 228
    #[serde(rename = "configInfo")]
    ConfigInfo(ConfigInfo),

    #[serde(rename = "tvm.cell")]
    TvmCell(TvmCell),
}

impl TonResult {
    pub fn expect_ok(&self) -> anyhow::Result<()> {
        match self {
            TonResult::Ok {} => Ok(()),
            r => Err(anyhow!("Expected Ok, got: {:?}", r)),
        }
    }
}
