use std::sync::Arc;

use super::{Cell, CellBuilder, TonCellError};

pub struct StateInit {
    pub code: Option<Arc<Cell>>,
    pub data: Option<Arc<Cell>>,
}

impl StateInit {
    pub fn new(code: &Arc<Cell>, data: &Arc<Cell>) -> StateInit {
        StateInit {
            code: Some(code.clone()),
            data: Some(data.clone()),
        }
    }

    pub fn build(&self) -> Result<Cell, TonCellError> {
        let mut builder = CellBuilder::new();
        builder
            .store_bit(false)? //Split depth
            .store_bit(false)? //Tick tock (always false)
            .store_bit(self.code.is_some())? //Code
            .store_bit(self.data.is_some())? //Data
            .store_bit(false)?; //Library
        if let Some(code) = &self.code {
            builder.store_reference(code)?;
        }
        if let Some(data) = &self.data {
            builder.store_reference(data)?;
        }
        builder.build()
    }

    pub fn create_account_id(code: &Arc<Cell>, data: &Arc<Cell>) -> Result<Vec<u8>, TonCellError> {
        Self::new(code, data).build()?.cell_hash()
    }
}
