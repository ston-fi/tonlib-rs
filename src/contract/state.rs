use std::sync::Arc;

use async_trait::async_trait;

use crate::address::TonAddress;
use crate::cell::Cell;
use crate::client::{TonClientError, TonClientInterface};
use crate::contract::{TonContractError, TonContractFactory, TonContractInterface};
use crate::emulator::{TvmEmulator, TvmEmulatorC7Builder};
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
        let method_id = &method.into();
        let stack_ref = stack.as_ref();
        let state = self.account_state.clone();
        let c7 = TvmEmulatorC7Builder::new(
            &self.address,
            self.factory.get_config_cell_serial().await?,
            0,
        )
        .build();

        let libs = self
            .factory
            .library_provider()
            .get_contract_libraries(&self.address, &self.account_state)
            .await?;

        let run_result = unsafe {
            // Using unsafe to extend lifetime of references to method_id & stack.
            //
            // This is necessary because the compiler doesn't have a proof that these references
            // outlive spawned future.
            // But we're know it for sure since we're awaiting it. In normal async/await block
            // this would be checked by the compiler, but not when using `spawn_blocking`
            let static_method_id: &'static TonMethodId = std::mem::transmute(method_id);
            let static_stack: &'static [TvmStackEntry] = std::mem::transmute(stack_ref);
            #[allow(clippy::let_and_return)]
            tokio::task::spawn_blocking(move || {
                let code = state.code.as_slice();
                let data = state.data.as_slice();
                let mut emulator = TvmEmulator::new(code, data)?;
                emulator.set_c7(&c7)?;
                emulator.set_libraries(libs.dict_boc.as_slice())?;
                let run_result = emulator.run_get_method(static_method_id, static_stack);
                run_result
            })
            .await
            .map_err(|e| TonContractError::InternalError(e.to_string()))?
        }
        .map_err(|e| TonContractError::MethodEmulationError {
            method: method_id.to_string(),
            address: self.address().clone(),
            error: e,
        });
        Self::raise_exit_error(self.address(), method_id, run_result?)
    }

    pub async fn emulate_internal_message(
        &self,
        message: Cell,
        amount: u64,
    ) -> Result<TvmMsgSuccess, TonContractError> {
        let state = self.account_state.clone();
        let c7 = TvmEmulatorC7Builder::new(
            &self.address,
            self.factory.get_config_cell_serial().await?,
            0,
        )
        .build();
        let run_result = tokio::task::spawn_blocking(move || {
            let code = state.code.as_slice();
            let data = state.data.as_slice();
            let mut emulator = TvmEmulator::new(code, data)?;
            emulator.set_c7(&c7)?;
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
                error: e,
            })?;

        let run_result = state
            .conn
            .smc_run_get_method(state.id, &method.into(), &stack_tl)
            .await?;

        let stack = run_result
            .stack
            .elements
            .iter()
            .map(|e| e.try_into())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| TonContractError::TvmStackParseError {
                method: method.into(),
                address: self.address().clone(),
                error: e,
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
                stack: run_result.stack,
                exit_code: run_result.vm_exit_code,
                vm_log: run_result.vm_log,
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
