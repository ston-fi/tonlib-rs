use crate::tl::{TlError, TonFunction, TonResultDiscriminants};
use crate::{client::connection::TonConnection, tl::TvmCell};
use crate::{config::MAINNET_CONFIG, tl::LiteServerInfo};
use async_trait::async_trait;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

use crate::tl::TonNotification;
use crate::tl::TonResult;
use crate::tl::{
    AccountAddress, BlockId, BlockIdExt, BlocksAccountTransactionId, BlocksHeader,
    BlocksMasterchainInfo, BlocksShards, BlocksTransactions, ConfigInfo, FullAccountState,
    InternalTransactionId, RawFullAccountState, RawTransactions,
};

use crate::client::TonClientError;

#[derive(Debug, Clone)]
pub struct TonError {
    pub code: i32,
    pub message: String,
}

impl fmt::Display for TonError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TonError code: {}, message: {}", self.code, self.message)
    }
}

impl Error for TonError {}

pub type TonNotificationReceiver = broadcast::Receiver<Arc<TonNotification>>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TonConnectionParams {
    pub config: String,
    pub blockchain_name: Option<String>,
    pub use_callbacks_for_network: bool,
    pub ignore_cache: bool,
    pub keystore_dir: Option<String>,
}

impl Default for TonConnectionParams {
    fn default() -> Self {
        TonConnectionParams {
            config: MAINNET_CONFIG.to_string(),
            blockchain_name: None,
            use_callbacks_for_network: false,
            ignore_cache: false,
            keystore_dir: None,
        }
    }
}

lazy_static! {
    pub static ref DEFAULT_CONNECTION_PARAMS: TonConnectionParams = TonConnectionParams::default();
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RetryStrategy {
    pub interval_ms: u64,
    pub max_retries: usize,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        RetryStrategy {
            interval_ms: 100,
            max_retries: 10,
        }
    }
}

lazy_static! {
    pub static ref DEFAULT_RETRY_STRATEGY: RetryStrategy = RetryStrategy::default();
}

#[allow(unused_variables)]
pub trait TonConnectionCallback {
    fn on_invoke(&self, id: u32) {}
    fn on_invoke_result(
        &self,
        id: u32,
        method: &str,
        duration: &Duration,
        res: &Result<TonResult, TonClientError>,
    ) {
    }
    fn on_invoke_result_send_error(
        &self,
        id: u32,
        duration: &Duration,
        res: &Result<TonResult, TonClientError>,
    ) {
    }
    fn on_notification(&self, notification: &TonNotification) {}
    fn on_tl_error(&self, error: &TlError) {}
    fn on_tonlib_error(&self, id: &Option<u32>, code: i32, error: &str) {}
    fn on_ton_result_parse_error(&self, result: &TonResult) {}
}

pub struct DefaultConnectionCallback {}
impl TonConnectionCallback for DefaultConnectionCallback {}

#[async_trait]
pub trait TonFunctions {
    async fn get_connection(&self) -> Result<TonConnection, TonClientError>;

    async fn invoke_on_connection(
        &self,
        function: &TonFunction,
    ) -> Result<(TonConnection, TonResult), TonClientError>;

    async fn invoke(&self, function: &TonFunction) -> Result<TonResult, TonClientError> {
        self.invoke_on_connection(function).await.map(|(_, r)| r)
    }

    async fn get_raw_account_state(
        &self,
        account_address: &str,
    ) -> Result<RawFullAccountState, TonClientError> {
        let func = TonFunction::RawGetAccountState {
            account_address: AccountAddress {
                account_address: String::from(account_address),
            },
        };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::RawFullAccountState(state) => Ok(state),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::RawFullAccountState,
                r,
            )),
        }
    }

    async fn get_raw_transactions(
        &self,
        account_address: &str,
        from_transaction_id: &InternalTransactionId,
    ) -> Result<RawTransactions, TonClientError> {
        let func = TonFunction::RawGetTransactions {
            account_address: AccountAddress {
                account_address: String::from(account_address),
            },
            from_transaction_id: from_transaction_id.clone(),
        };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::RawTransactions(state) => Ok(state),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::RawTransactions,
                r,
            )),
        }
    }

    async fn get_raw_transactions_v2(
        &self,
        account_address: &str,
        from_transaction_id: &InternalTransactionId,
        count: usize,
        try_decode_messages: bool,
    ) -> Result<RawTransactions, TonClientError> {
        let func = TonFunction::RawGetTransactionsV2 {
            account_address: AccountAddress {
                account_address: String::from(account_address),
            },
            from_transaction_id: from_transaction_id.clone(),
            count: count as u32,
            try_decode_messages,
        };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::RawTransactions(state) => Ok(state),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::RawTransactions,
                r,
            )),
        }
    }

    async fn send_raw_message(&self, body: &[u8]) -> Result<(), TonClientError> {
        let func = TonFunction::RawSendMessage {
            body: body.to_vec(),
        };
        self.invoke(&func).await?.expect_ok()
    }

    async fn send_raw_message_return_hash(&self, body: &[u8]) -> Result<Vec<u8>, TonClientError> {
        let func = TonFunction::RawSendMessageReturnHash {
            body: body.to_vec(),
        };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::RawExtMessageInfo(info) => Ok(info.hash),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::RawExtMessageInfo,
                r,
            )),
        }
    }

    async fn sync(&self) -> Result<(TonConnection, BlockIdExt), TonClientError> {
        let func = TonFunction::Sync {};
        let (conn, result) = self.invoke_on_connection(&func).await?;
        match result {
            TonResult::BlockIdExt(result) => Ok((conn, result)),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::BlockIdExt,
                r,
            )),
        }
    }

    async fn get_account_state(
        &self,
        account_address: &str,
    ) -> Result<FullAccountState, TonClientError> {
        let func = TonFunction::GetAccountState {
            account_address: AccountAddress {
                account_address: String::from(account_address),
            },
        };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::FullAccountState(state) => Ok(state),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::FullAccountState,
                r,
            )),
        }
    }

    async fn smc_load(
        &self,
        account_address: &str,
    ) -> Result<(TonConnection, i64), TonClientError> {
        let func = TonFunction::SmcLoad {
            account_address: AccountAddress {
                account_address: String::from(account_address),
            },
        };
        let (conn, result) = self.invoke_on_connection(&func).await?;
        match result {
            TonResult::SmcInfo(smc_info) => Ok((conn, smc_info.id)),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::SmcInfo,
                r,
            )),
        }
    }

    async fn smc_load_by_transaction(
        &self,
        account_address: &str,
        transaction_id: &InternalTransactionId,
    ) -> Result<(TonConnection, i64), TonClientError> {
        let func = TonFunction::SmcLoadByTransaction {
            account_address: AccountAddress {
                account_address: String::from(account_address),
            },
            transaction_id: transaction_id.clone(),
        };
        let (conn, result) = self.invoke_on_connection(&func).await?;
        match result {
            TonResult::SmcInfo(smc_info) => Ok((conn, smc_info.id)),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::SmcInfo,
                r,
            )),
        }
    }

    async fn smc_forget(&self, id: i64) -> Result<TonResult, TonClientError> {
        let func = TonFunction::SmcForget { id };
        let result = self.invoke(&func).await?;
        Ok(result)
    }

    async fn smc_get_code(&self, id: i64) -> Result<TvmCell, TonClientError> {
        let func = TonFunction::SmcGetCode { id: id };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::TvmCell(cell) => Ok(cell),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::TvmCell,
                r,
            )),
        }
    }

    async fn smc_get_data(&self, id: i64) -> Result<TvmCell, TonClientError> {
        let func = TonFunction::SmcGetData { id: id };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::TvmCell(cell) => Ok(cell),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::TvmCell,
                r,
            )),
        }
    }

    async fn smc_get_state(&self, id: i64) -> Result<TvmCell, TonClientError> {
        let func = TonFunction::SmcGetState { id: id };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::TvmCell(cell) => Ok(cell),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::TvmCell,
                r,
            )),
        }
    }

    async fn get_masterchain_info(&self) -> Result<BlocksMasterchainInfo, TonClientError> {
        let func = TonFunction::BlocksGetMasterchainInfo {};
        let result = self.invoke(&func).await?;
        match result {
            TonResult::BlocksMasterchainInfo(result) => Ok(result),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::BlocksMasterchainInfo,
                r,
            )),
        }
    }

    async fn get_block_shards(
        &self,
        block_id: &BlockIdExt,
    ) -> Result<BlocksShards, TonClientError> {
        let func = TonFunction::BlocksGetShards {
            id: block_id.clone(),
        };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::BlocksShards(result) => Ok(result),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::BlocksShards,
                r,
            )),
        }
    }

    /// Attempts to find block by specified query.
    ///
    /// * `mode`: Lookup mode: `1` - by `block_id.seqno`, `2` - by `lt`, `4` - by `utime`.
    async fn lookup_block(
        &self,
        mode: i32,
        block_id: &BlockId,
        lt: i64,
        utime: i32,
    ) -> Result<BlockIdExt, TonClientError> {
        let func = TonFunction::BlocksLookupBlock {
            mode,
            id: block_id.clone(),
            lt,
            utime,
        };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::BlockIdExt(result) => Ok(result),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::BlockIdExt,
                r,
            )),
        }
    }

    /// Returns up to specified number of ids of transactions in specified block.
    ///
    /// * `block_id`: ID of the block to retrieve transactions for (either masterchain or shard).
    /// * `mode`: Use `7` to get first chunk of transactions or `7 + 128` for subsequent chunks.
    /// * `count`: Maximum mumber of transactions to retrieve.
    /// * `after`: Specify `NULL_BLOCKS_ACCOUNT_TRANSACTION_ID` to get the first chunk
    ///             or id of the last retrieved tx for subsequent chunks.
    ///
    async fn get_block_transactions(
        &self,
        block_id: &BlockIdExt,
        mode: u32,
        count: u32,
        after: &BlocksAccountTransactionId,
    ) -> Result<BlocksTransactions, TonClientError> {
        let func = TonFunction::BlocksGetTransactions {
            id: block_id.clone(),
            mode,
            count,
            after: after.clone(),
        };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::BlocksTransactions(result) => Ok(result),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::BlocksTransactions,
                r,
            )),
        }
    }

    async fn lite_server_get_info(&self) -> Result<LiteServerInfo, TonClientError> {
        let func = TonFunction::LiteServerGetInfo {};
        let result = self.invoke(&func).await?;
        match result {
            TonResult::LiteServerInfo(result) => Ok(result),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::LiteServerInfo,
                r,
            )),
        }
    }

    async fn get_block_header(
        &self,
        block_id: &BlockIdExt,
    ) -> Result<BlocksHeader, TonClientError> {
        let func = TonFunction::GetBlockHeader {
            id: block_id.clone(),
        };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::BlocksHeader(header) => Ok(header),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::BlocksHeader,
                r,
            )),
        }
    }

    async fn get_config_param(&self, mode: u32, param: u32) -> Result<ConfigInfo, TonClientError> {
        let func = TonFunction::GetConfigParam { mode, param };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::ConfigInfo(result) => Ok(result),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::ConfigInfo,
                r,
            )),
        }
    }

    async fn get_log_verbosity_level(&self) -> Result<u32, TonClientError> {
        let func = TonFunction::GetLogVerbosityLevel {};
        let result = self.invoke(&func).await?;
        match result {
            TonResult::LogVerbosityLevel(log_verbosity_level) => {
                Ok(log_verbosity_level.verbosity_level)
            }
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::OptionsInfo,
                r,
            )),
        }
    }
}
