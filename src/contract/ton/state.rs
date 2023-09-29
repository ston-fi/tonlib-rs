use crate::tl::{SmcMethodId, SmcRunResult};
use crate::{address::TonAddress, tl::InternalTransactionId};
use crate::{client::TonConnection, tl::TvmStackEntry};
use crate::{client::TonFunctions, tl::TvmCell};

use crate::contract::TonContractError;

pub struct TonContractState {
    connection: TonConnection,
    state_id: i64,
}

impl TonContractState {
    pub async fn load<C: TonFunctions + Send + Sync>(
        client: &C,
        address: &TonAddress,
    ) -> Result<TonContractState, TonContractError> {
        let (conn, state_id) = client
            .smc_load(&address.to_hex())
            .await
            .map_err(|e| TonContractError::client_method_error("smc_load", Some(&address), e))?;
        Ok(TonContractState {
            connection: conn,
            state_id,
        })
    }
    pub async fn load_by_transaction_id<C: TonFunctions + Send + Sync>(
        client: &C,
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
            state_id,
        })
    }
    pub async fn run_get_method(
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
