use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use tonlib_core::cell::{ArcCell, BagOfCells, Cell};
use tonlib_core::library_helper::{ContractLibraryDict, LibraryHelper};
use tonlib_core::{TonAddress, TonHash};

use super::MapCellError;
use crate::client::{TonClientError, TonClientInterface};
use crate::contract::{TonContractError, TonContractFactory, TonContractInterface};
use crate::emulator::c7_register::TvmEmulatorC7;
use crate::emulator::error::TvmEmulatorError;
use crate::emulator::tvm_emulator::TvmEmulator;
use crate::tl::RawFullAccountState;
use crate::types::{TonMethodId, TvmMsgSuccess, TvmStackEntry, TvmSuccess};

#[derive(Clone)]
pub struct TonContractState {
    factory: TonContractFactory,
    address: TonAddress,
    account_state: Arc<RawFullAccountState>,
}

impl TonContractState {
    pub fn new(
        factory: &TonContractFactory,
        address: &TonAddress,
        account_state: &Arc<RawFullAccountState>,
    ) -> TonContractState {
        TonContractState {
            factory: factory.clone(),
            address: address.clone(),
            account_state: account_state.clone(),
        }
    }

    pub fn get_account_state(&self) -> &Arc<RawFullAccountState> {
        &self.account_state
    }

    #[cfg(feature = "emulate_get_method")]
    async fn do_run_get_method<M, S>(
        &self,
        method: M,
        stack: S,
    ) -> Result<TvmSuccess, TonContractError>
    where
        M: Into<TonMethodId> + Send + Copy,
        S: AsRef<[TvmStackEntry]> + Send,
    {
        let run_result = self.emulate_get_method(method, stack.as_ref()).await;

        match run_result {
            Ok(result) => Ok(result),
            Err(e) => {
                log::warn!(
                    "Contract emulator returned error: {} \n Falling back to tonlib_run_get_method",
                    e
                );
                self.tonlib_run_get_method(method, stack).await
            }
        }
    }

    #[cfg(not(feature = "emulate_get_method"))]
    async fn do_run_get_method<M, S>(
        &self,
        method: M,
        stack: S,
    ) -> Result<TvmSuccess, TonContractError>
    where
        M: Into<TonMethodId> + Send + Copy,
        S: AsRef<[TvmStackEntry]> + Send,
    {
        self.tonlib_run_get_method(method, stack).await
    }

    pub async fn emulate_get_method<M, S>(
        &self,
        method: M,
        stack: S,
    ) -> Result<TvmSuccess, TonContractError>
    where
        M: Into<TonMethodId> + Send + Copy,
        S: AsRef<[TvmStackEntry]> + Send,
    {
        let method_id: TonMethodId = method.into();
        let method_str = method_id.as_str();
        let stack_ref = stack.as_ref();
        let state = self.account_state.clone();
        let c7 = TvmEmulatorC7::new(
            self.address.clone(),
            self.factory.get_config_cell_serial().await?.to_vec(),
        )?;

        let code_hash = BagOfCells::parse(&self.account_state.code)
            .and_then(|boc| boc.single_root())
            .map_cell_error(method_str.clone(), &self.address)?
            .cell_hash();

        let mut libs = self
            .factory
            .library_provider()
            .get_or_load_libs(HashSet::from([code_hash.clone()]))
            .await?;

        let libs_dict = LibraryHelper::store_to_dict(libs.clone())?;
        let mut run_result = self
            .run_emulation_unsafe(
                state.code.as_slice(),
                state.data.as_slice(),
                c7.clone(),
                libs_dict,
                stack_ref,
                method_id.clone(),
            )
            .await?;

        let mut iteration = 0;
        while let Some(missing_lib_str) = &run_result.missing_library {
            if iteration > self.factory.max_libs_per_contract() {
                return Err(TonContractError::TooManyLibraries {
                    limit: self.factory.max_libs_per_contract(),
                    method: method_id.clone(),
                    address: self.address().clone(),
                });
            }
            iteration += 1;

            let missing_lib_id = TonHash::from_hex(missing_lib_str)
                .map_err(|e| TonContractError::InternalError(e.to_string()))?;
            let lib = self
                .factory
                .library_provider()
                .get_or_load_libs(HashSet::from([missing_lib_id.clone()]))
                .await?;
            if lib.is_empty() {
                return Err(TonContractError::MissingLibrary {
                    method: method_id.clone(),
                    address: self.address().clone(),
                    missing_library: missing_lib_str.to_string(),
                });
            };
            self.factory
                .library_provider()
                .update_code_libs(code_hash.clone(), missing_lib_id.clone());

            libs.extend(lib);
            let libs_dict = LibraryHelper::store_to_dict(libs.clone())?;
            run_result = self
                .run_emulation_unsafe(
                    state.code.as_slice(),
                    state.data.as_slice(),
                    c7.clone(),
                    libs_dict,
                    stack_ref,
                    method_id.clone(),
                )
                .await?;
        }

        Self::raise_exit_error(self.address(), &method_id, run_result)
    }

    async fn run_emulation_unsafe(
        &self,
        code: &[u8],
        data: &[u8],
        c7: TvmEmulatorC7,
        libs: ContractLibraryDict,
        stack: &[TvmStackEntry],
        method: TonMethodId,
    ) -> Result<TvmSuccess, TonContractError> {
        unsafe {
            // Using unsafe to extend lifetime of references to method_id & stack.
            //
            // This is necessary because the compiler doesn't have a proof that these references
            // outlive spawned future.
            // But we're know it for sure since we're awaiting it. In normal async/await block
            // this would be checked by the compiler, but not when using `spawn_blocking`

            let code_static: &'static [u8] = std::mem::transmute(code);
            let data_static: &'static [u8] = std::mem::transmute(data);
            let method_static: TonMethodId = method.clone();
            let static_stack: &'static [TvmStackEntry] = std::mem::transmute(stack);
            #[allow(clippy::let_and_return)]
            let res = tokio::task::spawn_blocking(move || {
                let mut emulator = TvmEmulator::new(code_static, data_static)?;
                emulator.with_c7(&c7)?.with_libraries(libs.0.as_slice())?;
                let run_result = emulator.run_get_method(&method_static, static_stack);
                run_result
            })
            .await
            .map_err(|e| TonContractError::InternalError(e.to_string()))?;

            res.map_err(|error| TonContractError::MethodEmulationError {
                method: method.to_string(),
                address: self.address().clone(),
                error,
            })
        }
    }

    pub async fn emulate_internal_message(
        &self,
        message: Cell,
        amount: u64,
    ) -> Result<TvmMsgSuccess, TonContractError> {
        let state = self.account_state.clone();
        let c7 = TvmEmulatorC7::new(
            self.address.clone(),
            self.factory.get_config_cell_serial().await?.to_vec(),
        )?;
        let run_result = tokio::task::spawn_blocking(move || {
            let code = state.code.as_slice();
            let data = state.data.as_slice();
            let mut emulator = TvmEmulator::new(code, data)?;
            emulator.with_c7(&c7)?;
            emulator.send_internal_message(message, amount)
        })
        .await
        .map_err(|e| TonContractError::InternalError(e.to_string()))?
        .map_err(|e| TonContractError::MessageEmulationError {
            address: self.address().clone(),
            error: e,
        });
        run_result
    }

    pub async fn tonlib_run_get_method<M, S>(
        &self,
        method: M,
        stack: S,
    ) -> Result<TvmSuccess, TonContractError>
    where
        M: Into<TonMethodId> + Send + Copy,
        S: AsRef<[TvmStackEntry]> + Send,
    {
        let address = &self.address;
        let transaction_id = &self.account_state.last_transaction_id;

        let maybe_state = self
            .factory()
            .get_smc_state_by_transaction(address, transaction_id)
            .await;
        // this fallback is not necessary
        let state = match maybe_state {
            Ok(state) => Ok(state),
            Err(TonContractError::ClientError(TonClientError::TonlibError { .. })) => {
                Ok(Arc::new(self.factory.client().smc_load(address).await?))
            }
            Err(e) => Err(e),
        }?;

        let stack_tl = stack
            .as_ref()
            .iter()
            .map(|e| e.try_into())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TonContractError::TvmStackParseError {
                method: method.into(),
                address: self.address().clone(),
                error: Box::new(e),
            })?;

        let run_result = state
            .conn
            .smc_run_get_method(state.id, &method.into(), &stack_tl)
            .await?;

        // TODO make it properly using drop!
        tokio::spawn(async move { state.conn.smc_forget(state.id).await });

        let stack = run_result
            .stack
            .elements
            .iter()
            .map(|e| e.try_into())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TonContractError::TvmStackParseError {
                method: method.into(),
                address: self.address().clone(),
                error: Box::new(e),
            })?;
        let result = TvmSuccess {
            vm_log: None,
            vm_exit_code: run_result.exit_code,
            stack,
            missing_library: None,
            gas_used: run_result.gas_used as i32,
        };
        Self::raise_exit_error(self.address(), &method.into(), result)
    }

    #[allow(clippy::result_large_err)]
    fn raise_exit_error(
        address: &TonAddress,
        method: &TonMethodId,
        run_result: TvmSuccess,
    ) -> Result<TvmSuccess, TonContractError> {
        if run_result.exit_error() {
            Err(TonContractError::TvmRunError {
                method: method.clone(),
                address: address.clone(),
                gas_used: run_result.gas_used.into(),
                stack: Box::new(run_result.stack),
                exit_code: run_result.vm_exit_code,
                vm_log: Box::new(run_result.vm_log),
                missing_library: run_result.missing_library,
            })
        } else {
            Ok(run_result)
        }
    }
}

#[async_trait]
impl TonContractInterface for TonContractState {
    fn factory(&self) -> &TonContractFactory {
        &self.factory
    }

    fn address(&self) -> &TonAddress {
        &self.address
    }

    async fn get_account_state(&self) -> Result<Arc<RawFullAccountState>, TonContractError> {
        Ok(self.account_state.clone())
    }

    async fn run_get_method<M, S>(
        &self,
        method: M,
        stack: S,
    ) -> Result<TvmSuccess, TonContractError>
    where
        M: Into<TonMethodId> + Send + Copy,
        S: AsRef<[TvmStackEntry]> + Send,
    {
        self.do_run_get_method(method, stack).await
    }
}
