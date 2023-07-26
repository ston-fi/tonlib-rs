use crate::address::TonAddress;
use crate::client::{TonConnection, TonFunctions};
use crate::contract::TonContractError;
use crate::tl::stack::TvmStackEntry;
use crate::tl::types::{SmcMethodId, SmcRunResult};

pub struct TonContractState {
    connection: TonConnection,
    state_id: i64,
}

impl TonContractState {
    pub(crate) async fn load<C: TonFunctions + Send + Sync>(
        client: &C,
        address: &TonAddress,
    ) -> anyhow::Result<TonContractState> {
        let (conn, state_id) = client.smc_load(&address.to_hex()).await?;
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
}

#[cfg(feature = "smc_forget")]
impl Drop for TonContractState {
    fn drop(&mut self) {
        let conn = self.connection.clone();
        let state_id = self.state_id;
        tokio::spawn(async move {
            let _ = conn.smc_forget(state_id).await; // Ignore failure
        });
    }
}
