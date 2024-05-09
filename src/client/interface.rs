use async_trait::async_trait;

use super::{SmcLibraryQueryExt, SmcLibraryResult, SmcLibraryResultExt, TonLibraryId};
use crate::address::TonAddress;
use crate::client::{TonClientError, TonConnection};
use crate::contract::LoadedSmcState;
use crate::tl::{
    AccountAddress, BlockId, BlockIdExt, BlocksAccountTransactionId, BlocksHeader,
    BlocksMasterchainInfo, BlocksShards, BlocksTransactions, BlocksTransactionsExt, ConfigInfo,
    FullAccountState, InternalTransactionId, LiteServerInfo, RawFullAccountState, RawTransactions,
    TonFunction, TonResult, TonResultDiscriminants, TvmCell,
};

#[async_trait]
pub trait TonClientInterface: Send + Sync {
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
        account_address: &TonAddress,
    ) -> Result<RawFullAccountState, TonClientError> {
        let func = TonFunction::RawGetAccountState {
            account_address: AccountAddress {
                account_address: account_address.to_hex(),
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

    async fn get_raw_account_state_by_transaction(
        &self,
        account_address: &TonAddress,
        transaction_id: &InternalTransactionId,
    ) -> Result<RawFullAccountState, TonClientError> {
        let func = TonFunction::RawGetAccountStateByTransaction {
            account_address: AccountAddress {
                account_address: account_address.to_hex(),
            },
            transaction_id: transaction_id.clone(),
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
        account_address: &TonAddress,
        from_transaction_id: &InternalTransactionId,
    ) -> Result<RawTransactions, TonClientError> {
        let func = TonFunction::RawGetTransactions {
            account_address: AccountAddress {
                account_address: account_address.to_hex(),
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
        account_address: &TonAddress,
        from_transaction_id: &InternalTransactionId,
        count: usize,
        try_decode_messages: bool,
    ) -> Result<RawTransactions, TonClientError> {
        let func = TonFunction::RawGetTransactionsV2 {
            account_address: AccountAddress {
                account_address: account_address.to_hex(),
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
        account_address: &TonAddress,
    ) -> Result<FullAccountState, TonClientError> {
        let func = TonFunction::GetAccountState {
            account_address: AccountAddress {
                account_address: account_address.to_hex(),
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
        account_address: &TonAddress,
    ) -> Result<LoadedSmcState, TonClientError> {
        let func = TonFunction::SmcLoad {
            account_address: AccountAddress {
                account_address: account_address.to_hex(),
            },
        };
        let (conn, result) = self.invoke_on_connection(&func).await?;
        match result {
            TonResult::SmcInfo(smc_info) => Ok(LoadedSmcState {
                conn,
                id: smc_info.id,
            }),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::SmcInfo,
                r,
            )),
        }
    }
    async fn smc_load_by_transaction(
        &self,
        address: &TonAddress,
        tx_id: &InternalTransactionId,
    ) -> Result<LoadedSmcState, TonClientError> {
        let func = TonFunction::SmcLoadByTransaction {
            account_address: AccountAddress {
                account_address: address.to_hex(),
            },
            transaction_id: tx_id.clone(),
        };
        let (conn, result) = self.invoke_on_connection(&func).await?;
        match result {
            TonResult::SmcInfo(smc_info) => Ok(LoadedSmcState {
                conn,
                id: smc_info.id,
            }),
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
        let func = TonFunction::SmcGetCode { id };
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
        let func = TonFunction::SmcGetData { id };
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
        let func = TonFunction::SmcGetState { id };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::TvmCell(cell) => Ok(cell),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::TvmCell,
                r,
            )),
        }
    }

    async fn smc_get_libraries(
        &self,
        library_list: &[TonLibraryId],
    ) -> Result<SmcLibraryResult, TonClientError> {
        let func = TonFunction::SmcGetLibraries {
            library_list: library_list.to_vec(),
        };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::SmcLibraryResult(r) => Ok(r),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::SmcLibraryResult,
                r,
            )),
        }
    }

    async fn smc_get_libraries_ext(
        &self,
        list: &[SmcLibraryQueryExt],
    ) -> Result<SmcLibraryResultExt, TonClientError> {
        let func = TonFunction::SmcGetLibrariesExt {
            list: list.to_vec(),
        };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::SmcLibraryResultExt(r) => Ok(r),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::SmcLibraryResultExt,
                r,
            )),
        }
    }

    async fn get_masterchain_info(
        &self,
    ) -> Result<(TonConnection, BlocksMasterchainInfo), TonClientError> {
        let func = TonFunction::BlocksGetMasterchainInfo {};
        let (conn, result) = self.invoke_on_connection(&func).await?;
        match result {
            TonResult::BlocksMasterchainInfo(result) => Ok((conn, result)),
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

    async fn get_block_transactions_ext(
        &self,
        block_id: &BlockIdExt,
        mode: u32,
        count: u32,
        after: &BlocksAccountTransactionId,
    ) -> Result<BlocksTransactionsExt, TonClientError> {
        let func = TonFunction::BlocksGetTransactionsExt {
            id: block_id.clone(),
            mode,
            count,
            after: after.clone(),
        };
        let result = self.invoke(&func).await?;
        match result {
            TonResult::BlocksTransactionsExt(result) => Ok(result),
            r => Err(TonClientError::unexpected_ton_result(
                TonResultDiscriminants::BlocksTransactionsExt,
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

    async fn get_config_all(&self, mode: u32) -> Result<ConfigInfo, TonClientError> {
        let func = TonFunction::GetConfigAll { mode };
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
