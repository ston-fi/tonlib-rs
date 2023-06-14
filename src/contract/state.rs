use crate::client::{TonConnection, TonFunctions};
use crate::contract::TonContractError;
use crate::tl::stack::TvmStackEntry;
use crate::tl::types::{SmcMethodId, SmcRunResult};
use crate::{address::TonAddress, tl::TonResult};

pub struct TonContractState<'a, C>
where
    C: TonFunctions + Send + Sync,
{
    connection: TonConnection,
    state_id: i64,
    client: &'a C,
}

impl<'a, T> TonContractState<'a, T>
where
    T: TonFunctions + Send + Sync,
{
    pub(crate) async fn load(
        client: &'a T,
        address: &TonAddress,
    ) -> anyhow::Result<TonContractState<'a, T>> {
        let (conn, state_id) = client.smc_load(&address.to_hex()).await?;
        Ok(TonContractState {
            client,
            connection: conn,
            state_id,
        })
    }

    pub async fn forget(&self, client: &'a T) -> anyhow::Result<TonResult> {
        client.smc_forget(self.state_id).await
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

impl<T: TonFunctions + Send + Sync> Drop for TonContractState<'_, T> {
    fn drop(&mut self) {
        let _ = self.forget(self.client);
    }
}
