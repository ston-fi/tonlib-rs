use tonlib_core::cell::ArcCell;

use crate::types::TvmStackEntry;

#[derive(Debug)]
pub struct TvmSuccess {
    pub vm_log: Option<String>,
    pub vm_exit_code: i32,
    pub stack: Vec<TvmStackEntry>,
    pub missing_library: Option<String>,
    pub gas_used: i32,
}

"actions"
#[derive(Debug)]
pub struct TvmMsgSuccess {
    pub new_code: ArcCell,
    pub new_data: ArcCell,
    pub accepted: bool,
    pub vm_exit_code: i32,
    pub vm_log: Option<String>,
    pub missing_library: Option<String>,
    pub gas_used: i32,
    pub actions: Option<ArcCell>,
}
