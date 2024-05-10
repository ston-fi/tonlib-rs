use std::fmt;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumDiscriminants, IntoStaticStr};

use crate::client::TonClientError;
use crate::tl::stack::TvmCell;
use crate::tl::types::{
    BlockIdExt, BlocksHeader, BlocksMasterchainInfo, BlocksShards, BlocksTransactions,
    BlocksTransactionsExt, ConfigInfo, FullAccountState, LiteServerInfo, LogVerbosityLevel,
    OptionsInfo, RawExtMessageInfo, RawFullAccountState, RawTransactions, SmcInfo,
    SmcLibraryResult, SmcLibraryResultExt, SmcRunResult, UpdateSyncState,
};

#[derive(
    IntoStaticStr, EnumDiscriminants, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash,
)]
#[strum_discriminants(derive(IntoStaticStr, Display))]
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
    // tonlib_api.tl, line 90
    #[serde(rename = "fullAccountState")]
    FullAccountState(FullAccountState),
    // tonlib_api.tl, line 167
    #[serde(rename = "tvm.cell")]
    TvmCell(TvmCell),
    // tonlib_api.tl, line 179
    #[serde(rename = "smc.info")]
    SmcInfo(SmcInfo),
    // tonlib_api.tl, line 184
    #[serde(rename = "smc.runResult")]
    SmcRunResult(SmcRunResult),
    // tonlib_api.tl, line 187
    #[serde(rename = "smc.libraryResult")]
    SmcLibraryResult(SmcLibraryResult),
    // tonlib_api.tl, line 191
    #[serde(rename = "smc.libraryResultExt")]
    SmcLibraryResultExt(SmcLibraryResultExt),
    // tonlib_api.tl, line 194
    #[serde(rename = "updateSyncState")]
    UpdateSyncState(UpdateSyncState),
    // tonlib_api.tl, line 203
    #[serde(rename = "liteServer.info")]
    LiteServerInfo(LiteServerInfo),
    // tonlib_api.tl, line 216
    #[serde(rename = "logVerbosityLevel")]
    LogVerbosityLevel(LogVerbosityLevel),
    // tonlib_api.tl, line 219
    #[serde(rename = "blocks.masterchainInfo")]
    BlocksMasterchainInfo(BlocksMasterchainInfo),
    // tonlib_api.tl, line 220
    #[serde(rename = "blocks.shards")]
    BlocksShards(BlocksShards),
    // tonlib_api.tl, line 223
    #[serde(rename = "blocks.transactions")]
    BlocksTransactions(BlocksTransactions),
    // tonlib_api.tl, line 224
    #[serde(rename = "blocks.transactionsExt")]
    BlocksTransactionsExt(BlocksTransactionsExt),
    // tonlib_api.tl, line 225
    #[serde(rename = "blocks.header")]
    BlocksHeader(BlocksHeader),
    // tonlib_api.tl, line 243
    #[serde(rename = "configInfo")]
    ConfigInfo(ConfigInfo),
}

impl TonResult {
    pub fn expect_ok(&self) -> Result<(), TonClientError> {
        match self {
            TonResult::Ok {} => Ok(()),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::Ok,
                r.clone(),
            )),
        }
    }
}
impl fmt::Display for TonResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // tonlib_api.tl, line 20
            TonResult::Error { code, message } => {
                write!(f, "TonResult::Error {}: {} ", code, message)
            }

            TonResult::Ok {} => write!(f, "TonResult::Ok"),

            TonResult::OptionsInfo(options_info) => write!(
                f,
                "TonResult::OptionsInfo: {}",
                options_info.config_info.default_wallet_id
            ),

            TonResult::BlockIdExt(block_id_ext) => write!(
                f,
                "TonResult::BlockIdExt: {}:{}, seqno{}",
                block_id_ext.workchain, block_id_ext.shard, block_id_ext.seqno
            ),

            TonResult::RawFullAccountState(raw_full_account_state) => write!(
                f,
                "TonResult::RawFullAccountState: {}:{}, seqno{}, last_td_id {}",
                raw_full_account_state.block_id.workchain,
                raw_full_account_state.block_id.shard,
                raw_full_account_state.block_id.seqno,
                raw_full_account_state.last_transaction_id
            ),

            TonResult::RawTransactions(raw_transactions) => write!(
                f,
                "TonResult::RawTransactions: prev_tx_id {}",
                raw_transactions.previous_transaction_id
            ),

            TonResult::RawExtMessageInfo(_) => write!(f, "TonResult::RawExtMessageInfo"),

            TonResult::FullAccountState(full_account_state) => write!(
                f,
                "TonResult::FullAccountState: address: {}",
                full_account_state.address.account_address
            ),

            TonResult::SmcInfo(_) => write!(f, "TonResult::SmcInfo"),

            TonResult::SmcRunResult(smc_run_result) => {
                write!(f, "TonResult::SmcRunResult: {}", smc_run_result.exit_code)
            }

            TonResult::SmcLibraryResult(smc_library_result_ext)=>write!(
                f,
                "TonResult::SmcLibraryResult: {:?}",
                smc_library_result_ext),

            TonResult::SmcLibraryResultExt(smc_library_result_ext) => write!(
                f,
                "TonResult::SmcLibraryResultExt: Raw dictionary: {:?}\n Libs ok: {:?}\n Libs not found:{:?}",
                smc_library_result_ext.dict_boc,
                smc_library_result_ext.libs_ok,
                smc_library_result_ext.libs_not_found
            ),

            TonResult::UpdateSyncState(_) => write!(f, "TonResult::UpdateSyncState"),

            TonResult::LiteServerInfo(_) => write!(f, "TonResult::LiteServerInfo"),

            TonResult::LogVerbosityLevel(log_verbosity_level) => write!(
                f,
                "TonResult::LogVerbosityLevel: {}",
                log_verbosity_level.verbosity_level
            ),

            TonResult::BlocksMasterchainInfo(blocks_masterchain_info) => write!(
                f,
                "TonResult::BlocksMasterchainInfo: {}:{}, seqno{}",
                blocks_masterchain_info.last.workchain,
                blocks_masterchain_info.last.shard,
                blocks_masterchain_info.last.seqno
            ),

            TonResult::BlocksShards(_) => write!(f, "TonResult::BlocksShards"),

            TonResult::BlocksTransactions(blocks_trasnactions) => write!(
                f,
                "TonResult::BlocksTransactions: {}:{}, seqno{}",
                blocks_trasnactions.id.workchain,
                blocks_trasnactions.id.shard,
                blocks_trasnactions.id.seqno
            ),

            TonResult::BlocksTransactionsExt(blocks_trasnactions) => write!(
                f,
                "TonResult::BlocksTransactions: {}:{}, seqno{}",
                blocks_trasnactions.id.workchain,
                blocks_trasnactions.id.shard,
                blocks_trasnactions.id.seqno
            ),

            TonResult::BlocksHeader(blocks_header) => write!(
                f,
                "TonResult::BlocksHeader: {}:{}, seqno{}",
                blocks_header.id.workchain, blocks_header.id.shard, blocks_header.id.seqno
            ),

            TonResult::ConfigInfo(_) => write!(f, "TonResult::ConfigInfo"),

            TonResult::TvmCell(_) => write!(f, "TonResult::TvmCell"),
        }
    }
}
