use crate::contract::TonContractError;
use crate::tl::stack::TvmStackEntry;
use crate::tl::types::{SmcMethodId, SmcRunResult};
use crate::{address::TonAddress, tl::types::InternalTransactionId};
use crate::{
    client::{TonConnection, TonFunctions},
    tl::stack::TvmCell,
};

pub struct TonContractState {
    connection: TonConnection,
    state_id: i64,
}

impl TonContractState {
    pub async fn load<C: TonFunctions + Send + Sync>(
        client: &C,
        address: &TonAddress,
    ) -> anyhow::Result<TonContractState> {
        let (conn, state_id) = client.smc_load(&address.to_hex()).await?;
        Ok(TonContractState {
            connection: conn,
            state_id,
        })
    }
    pub async fn load_by_transaction_id<C: TonFunctions + Send + Sync>(
        client: &C,
        address: &TonAddress,
        transaction_id: &InternalTransactionId,
    ) -> anyhow::Result<TonContractState> {
        let (conn, state_id) = client
            .smc_load_by_transaction(&address.to_hex(), transaction_id)
            .await?;
        Ok(TonContractState {
            connection: conn,
            state_id,
        })
    }
    pub async fn run_get_method(
        &self,
        method: &str,
        stack: &Vec<TvmStackEntry>,
    ) -> anyhow::Result<SmcRunResult> {
        let method = SmcMethodId::Name {
            name: String::from(method),
        };
        let result = self
            .connection
            .smc_run_get_method(self.state_id, &method, stack)
            .await?;
        if result.exit_code == 0 || result.exit_code == 1 {
            Ok(result)
        } else {
            let err = TonContractError {
                gas_used: result.gas_used,
                stack: result.stack.elements,
                exit_code: result.exit_code,
            };
            Err(anyhow::Error::from(err))
        }
    }

    pub async fn get_code(&self) -> anyhow::Result<TvmCell> {
        let result = self.connection.smc_get_code(self.state_id).await?;
        Ok(result)
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
