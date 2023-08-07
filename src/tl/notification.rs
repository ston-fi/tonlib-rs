use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use crate::tl::result::TonResult;
use crate::tl::types::UpdateSyncState;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TonNotification {
    // tonlib_api.tl, line 188
    UpdateSyncState(UpdateSyncState),
}

impl TonNotification {
    pub fn from_result(r: &TonResult) -> anyhow::Result<TonNotification> {
        match r {
            TonResult::UpdateSyncState(sync_state) => {
                Ok(TonNotification::UpdateSyncState(sync_state.clone()))
            }
            _ => Err(anyhow!("Not a valid notification: {:?}", r)),
        }
    }
}
