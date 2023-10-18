use std::sync::Arc;

use crate::{address::TonAddress, tl::InternalTransactionId};
use crate::{
    client::TonClient,
    tl::{SmcMethodId, SmcRunResult},
};
use crate::{client::TonConnection, tl::TvmStackEntry};
use crate::{client::TonFunctions, tl::TvmCell};
use async_trait::async_trait;

use crate::contract::{TonContractError, TonContractInterface};

use super::MapClientError;

pub struct TonContractState {
    connection: TonConnection,
    address: TonAddress,
    client: Arc<TonClient>,
    state_id: i64,
}

impl TonContractState {
    pub async fn load(
        client: Arc<TonClient>,
        address: &TonAddress,
    ) -> Result<TonContractState, TonContractError> {
        let (conn, state_id) = client
            .smc_load(&address.to_hex())
            .await
            .map_err(|e| TonContractError::client_method_error("smc_load", Some(&address), e))?;
        Ok(TonContractState {
            connection: conn,
            address: address.clone(),
            client: client,
            state_id,
        })
    }
    pub async fn load_by_transaction_id(
        client: Arc<TonClient>,
        address: &TonAddress,
        transaction_id: &InternalTransactionId,
    ) -> Result<TonContractState, TonContractError> {
        let (conn, state_id) = client
            .smc_load_by_transaction(&address.to_hex(), transaction_id)
            .await
            .map_err(|error| {
                TonContractError::client_method_error(
                    "smc_load_by_transaction",
                    Some(&address),
                    error,
                )
            })?;
        Ok(TonContractState {
            connection: conn,
            address: address.clone(),
            client: client.clone(),
            state_id,
        })
    }

    pub async fn get_code(&self) -> Result<TvmCell, TonContractError> {
        let result = self
            .connection
            .smc_get_code(self.state_id)
            .await
            .map_err(|error| TonContractError::client_method_error("smc_get_code", None, error));
        result
    }
}

impl Drop for TonContractState {
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
    fn client(&self) -> &TonClient {
        &self.client
    }

    fn address(&self) -> &TonAddress {
        &self.address
    }

    async fn get_code(&self) -> Result<TvmCell, TonContractError> {
        self.connection
            .smc_get_code(self.state_id)
            .await
            .map_client_error("get_code", self.address())
    }

    async fn get_data(&self) -> Result<TvmCell, TonContractError> {
        self.connection
            .smc_get_data(self.state_id)
            .await
            .map_client_error("get_data", self.address())
    }

    async fn get_state(&self) -> Result<TvmCell, TonContractError> {
        self.connection
            .smc_get_state(self.state_id)
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
            .connection
            .smc_run_get_method(self.state_id, &method_id, stack)
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
