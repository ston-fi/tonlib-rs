use serde::{Deserialize, Serialize};

use crate::tl::result::TonResult;
use crate::tl::types::UpdateSyncState;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum TonNotification {
    // tonlib_api.tl, line 194
    UpdateSyncState(UpdateSyncState),
}

impl TonNotification {
    pub fn from_result(r: &TonResult) -> Option<TonNotification> {
        match r {
            TonResult::UpdateSyncState(sync_state) => {
                Some(TonNotification::UpdateSyncState(sync_state.clone()))
            }
            _ => None,
        }
    }
}
