use std::sync::Arc;

use crate::address::TonAddress;
use crate::client::{TonClientInterface, TonConnection};
use crate::tl::{InternalTransactionId, SmcMethodId, SmcRunResult, TvmCell, TvmStackEntry};
use async_trait::async_trait;

use crate::contract::{TonContractError, TonContractInterface};

use super::MapClientError;

struct Inner {
    address: TonAddress,
    connection: TonConnection,
    state_id: i64,
}

#[derive(Clone)]
pub struct TonContractState {
    inner: Arc<Inner>,
}

impl TonContractState {
    pub async fn load(
        client: &dyn TonClientInterface,
        address: &TonAddress,
    ) -> Result<TonContractState, TonContractError> {
        let (conn, state_id) = client
            .smc_load(&address.to_hex())
            .await
            .map_err(|e| TonContractError::client_method_error("smc_load", Some(&address), e))?;
        let inner = Inner {
            address: address.clone(),
            connection: conn,
            state_id,
        };
        Ok(TonContractState {
            inner: Arc::new(inner),
        })
    }

    pub async fn load_by_transaction_id(
        client: &dyn TonClientInterface,
        address: &TonAddress,
        transaction_id: &InternalTransactionId,
    ) -> Result<TonContractState, TonContractError> {
        let (conn, state_id) = client
            .smc_load_by_transaction(&address, transaction_id)
            .await
            .map_err(|error| {
                TonContractError::client_method_error(
                    "smc_load_by_transaction",
                    Some(&address),
                    error,
                )
            })?;
        let inner = Inner {
            address: address.clone(),
            connection: conn,
            state_id,
        };
        Ok(TonContractState {
            inner: Arc::new(inner),
        })
    }

    pub async fn get_code(&self) -> Result<TvmCell, TonContractError> {
        let result = self
            .inner
            .connection
            .smc_get_code(self.inner.state_id)
            .await
            .map_err(|error| TonContractError::client_method_error("smc_get_code", None, error));
        result
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
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

    async fn get_code(&self) -> Result<TvmCell, TonContractError> {
        self.inner
            .connection
            .smc_get_code(self.inner.state_id)
            .await
            .map_client_error("get_code", self.address())
    }

    async fn get_data(&self) -> Result<TvmCell, TonContractError> {
        self.inner
            .connection
            .smc_get_data(self.inner.state_id)
            .await
            .map_client_error("get_data", self.address())
    }

    async fn get_state(&self) -> Result<TvmCell, TonContractError> {
        self.inner
            .connection
            .smc_get_state(self.inner.state_id)
            .await
            .map_client_error("get_state", self.address())
    }

    async fn run_get_method(
        &self,
        method: &str,
        stack: &Vec<TvmStackEntry>,
    ) -> Result<SmcRunResult, TonContractError> {
        let method_id = SmcMethodId::Name {
            name: String::from(method),
        };
        let result = self
            .inner
            .connection
            .smc_run_get_method(self.inner.state_id, &method_id, stack)
            .await
            .map_err(|error| TonContractError::client_method_error(method, None, error))?;
        if result.exit_code == 0 || result.exit_code == 1 {
            Ok(result)
        } else {
            Err(TonContractError::TvmRunError {
                gas_used: result.gas_used,
                stack: result.stack.elements,
                exit_code: result.exit_code,
            })
        }
    }
}
