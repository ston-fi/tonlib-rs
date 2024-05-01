use std::ops::Neg;

use num_bigint::BigInt;
use serde::{Deserialize, Serialize};

use super::TvmEmulatorError;
use crate::cell::{BagOfCells, CellSlice};
use crate::types::{TvmMsgSuccess, TvmStackEntry, TvmSuccess};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct TvmEmulatorResponse {
    success: bool,
    vm_log: Option<String>,
    vm_exit_code: Option<i32>,
    stack: Option<String>,
    missing_library: Option<String>,
    gas_used: Option<String>,
    error: Option<String>,
}

#[allow(clippy::let_and_return)]
impl TvmEmulatorResponse {
    pub fn from_json(json_str: &str) -> Result<TvmSuccess, TvmEmulatorError> {
        let response: TvmEmulatorResponse = serde_json::from_str(json_str)?;

        let result = match response.success {
            true => {
                let vm_log = response
                    .vm_log
                    .ok_or(TvmEmulatorError::MissingJsonField("vm_log"))?;
                let vm_exit_code = response
                    .vm_exit_code
                    .ok_or(TvmEmulatorError::MissingJsonField("vm_exit_code"))?;
                let stack_string = response
                    .stack
                    .ok_or(TvmEmulatorError::MissingJsonField("stack"))?;
                let missing_library = response.missing_library;
                let gas_used = response
                    .gas_used
                    .ok_or(TvmEmulatorError::MissingJsonField("gas_used"))?
                    .parse::<i32>()
                    .map_err(|e| TvmEmulatorError::InternalError(e.to_string()))?;
                let boc = BagOfCells::parse_base64(stack_string.as_str())?;

                let stack = Self::extract_stack(&boc)?;

                Ok(TvmSuccess {
                    vm_log: Some(vm_log),
                    vm_exit_code,
                    stack,
                    missing_library,
                    gas_used,
                })
            }
            false => {
                let error = response
                    .error
                    .ok_or(TvmEmulatorError::MissingJsonField("error"))?;
                Err(TvmEmulatorError::EmulatorError(error))
            }
        };

        result
    }

    fn extract_stack(boc: &BagOfCells) -> Result<Vec<TvmStackEntry>, TvmEmulatorError> {
        let mut stack = vec![];

        let mut current_cell = boc.single_root()?;
        log::trace!("Parsing stack:\n{:?}", current_cell);

        let mut parser = current_cell.parser();

        let elements_count = parser.load_u32(24)?;

        for element in 0..elements_count {
            let element_type = parser.load_byte()?;

            let stack_entry = match element_type {
                0 => TvmStackEntry::Null,
                1 => TvmStackEntry::Int64(parser.load_i64(64)?),
                2 => match parser.load_byte()? {
                    0 => {
                        let bit_len = parser.remaining_bits();
                        let num = BigInt::from(parser.load_uint(bit_len)?);
                        TvmStackEntry::Int257(num)
                    }
                    1 => {
                        let bit_len = parser.remaining_bits();
                        let num = BigInt::from(parser.load_uint(bit_len)?).neg();
                        TvmStackEntry::Int257(num)
                    }
                    0xff => TvmStackEntry::Nan,
                    _ => TvmStackEntry::Unsupported,
                },
                3 => TvmStackEntry::Cell(current_cell.reference(1)?.clone()),
                4 => {
                    let st_bits = parser.load_u32(10)? as usize;
                    let end_bits = parser.load_u32(10)? as usize;
                    let st_ref = parser.load_u32(3)? as usize;
                    let end_ref = parser.load_u32(3)? as usize;

                    let cell = current_cell.reference(1)?;
                    let slice = CellSlice::new(cell, st_bits, end_bits, st_ref, end_ref)?;
                    TvmStackEntry::Slice(slice)
                }
                _ => TvmStackEntry::Unsupported,
            };
            // TODO: Remove trace when feature emulator is stable
            log::trace!(
                "element#{:?} ,type: {:?}:: {:?}",
                element,
                element_type,
                stack_entry
            );
            if element != elements_count - 1 {
                current_cell = current_cell.reference(0)?;
                parser = current_cell.parser();
            }
            stack.push(stack_entry);
        }
        stack.reverse();
        Ok(stack)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct TvmEmulatorMessageResponse {
    success: bool,
    new_code: Option<String>,
    new_data: Option<String>,
    accepted: Option<bool>,
    vm_exit_code: Option<i32>,
    vm_log: Option<String>,
    missing_library: Option<String>,
    gas_used: Option<String>,
    actions: Option<String>,
    error: Option<String>,
}

impl TvmEmulatorMessageResponse {
    pub fn from_json(json_str: &str) -> Result<TvmMsgSuccess, TvmEmulatorError> {
        let response: TvmEmulatorMessageResponse = serde_json::from_str(json_str)?;

        match response.success {
            true => {
                let new_code_string = response
                    .new_code
                    .ok_or(TvmEmulatorError::MissingJsonField("new_code"))?;
                let new_data_string = response
                    .new_data
                    .ok_or(TvmEmulatorError::MissingJsonField("new_data"))?;

                let accepted = response
                    .accepted
                    .ok_or(TvmEmulatorError::MissingJsonField("accepted"))?;

                let vm_log = response
                    .vm_log
                    .ok_or(TvmEmulatorError::MissingJsonField("vm_log"))?;
                let vm_exit_code = response
                    .vm_exit_code
                    .ok_or(TvmEmulatorError::MissingJsonField("vm_exit_code"))?;

                let missing_library = response.missing_library;

                let gas_used = response
                    .gas_used
                    .ok_or(TvmEmulatorError::MissingJsonField("gas_used"))?
                    .parse::<i32>()
                    .map_err(|e| TvmEmulatorError::InternalError(e.to_string()))?;

                let actions_sting = response.actions;

                let new_code = BagOfCells::parse_base64(&new_code_string)?
                    .single_root()?
                    .clone();
                let new_data = BagOfCells::parse_base64(&new_data_string)?
                    .single_root()?
                    .clone();

                let actions = if let Some(str) = actions_sting {
                    Some(BagOfCells::parse_base64(&str)?.single_root()?.clone())
                } else {
                    None
                };

                Ok(TvmMsgSuccess {
                    new_code,
                    new_data,
                    accepted,
                    vm_exit_code,
                    vm_log: Some(vm_log),
                    missing_library,
                    gas_used,
                    actions,
                })
            }
            false => {
                let error = response
                    .error
                    .ok_or(TvmEmulatorError::MissingJsonField("error"))?;
                Err(TvmEmulatorError::EmulatorError(error))
            }
        }
    }
}
