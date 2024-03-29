use std::sync::Arc;

use async_trait::async_trait;
use tokio::runtime::Handle;

use crate::address::TonAddress;
use crate::client::{TonClientInterface, TonConnection};
use crate::contract::{TonContractError, TonContractInterface};
use crate::tl::{InternalTransactionId, SmcRunResult, TvmCell, TvmStackEntry};
use crate::types::TonMethodId;

struct Inner {
    address: TonAddress,
    connection: TonConnection,
    state_id: i64,
    runtime: Handle,
}

#[derive(Clone)]
pub struct TonContractState {
    inner: Arc<Inner>,
}

impl TonContractState {
    pub(crate) async fn load(
        client: &dyn TonClientInterface,
        address: &TonAddress,
    ) -> Result<TonContractState, TonContractError> {
        let (conn, state_id) = client.smc_load(&address.to_hex()).await?;
        let inner = Inner {
            address: address.clone(),
            connection: conn,
            state_id,
            runtime: Handle::current(),
        };
        Ok(TonContractState {
            inner: Arc::new(inner),
        })
    }

    pub(crate) async fn load_by_transaction(
        client: &dyn TonClientInterface,
        address: &TonAddress,
        transaction_id: &InternalTransactionId,
    ) -> Result<TonContractState, TonContractError> {
        let (conn, state_id) = client
            .smc_load_by_transaction(address, transaction_id)
            .await?;
        let inner = Inner {
            address: address.clone(),
            connection: conn,
            state_id,
            runtime: Handle::current(),
        };
        Ok(TonContractState {
            inner: Arc::new(inner),
        })
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        let _guard = self.runtime.enter();
        let conn = self.connection.clone();
        let state_id = self.state_id;
        tokio::spawn(async move {
            let _ = conn.smc_forget(state_id).await; // Ignore failure
        });
    }
}

#[async_trait]
impl TonContractInterface for TonContractState {
    fn client(&self) -> &dyn TonClientInterface {
        &self.inner.connection
    }

    fn address(&self) -> &TonAddress {
        &self.inner.address
    }

    async fn get_code_cell(&self) -> Result<TvmCell, TonContractError> {
        let r = self
            .inner
            .connection
            .smc_get_code(self.inner.state_id)
            .await?;
        Ok(r)
    }

    async fn get_data_cell(&self) -> Result<TvmCell, TonContractError> {
        let r = self
            .inner
            .connection
            .smc_get_data(self.inner.state_id)
            .await?;
        Ok(r)
    }

    async fn get_state_cell(&self) -> Result<TvmCell, TonContractError> {
        let r = self
            .inner
            .connection
            .smc_get_state(self.inner.state_id)
            .await?;
        Ok(r)
    }

    async fn run_get_method<A: Into<TonMethodId> + Send>(
        &self,
        method: A,
        stack: &Vec<TvmStackEntry>,
    ) -> Result<SmcRunResult, TonContractError> {
        let method_id: TonMethodId = method.into();
        let result = self
            .inner
            .connection
            .smc_run_get_method(self.inner.state_id, &method_id, stack)
            .await?;
        if result.exit_code == 0 || result.exit_code == 1 {
            Ok(result)
        } else {
            Err(TonContractError::TvmRunError {
                gas_used: result.gas_used,
                method: method_id.clone(),
                stack: result.stack.elements,
                exit_code: result.exit_code,
            })
        }
    }
}
