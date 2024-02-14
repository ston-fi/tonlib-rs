use crate::cell::{Cell, CellBuilder, TonCellError};

/// WalletVersion::V1R1 | WalletVersion::V1R2 | WalletVersion::V1R3 | WalletVersion::V2R1 | WalletVersion::V2R2
pub struct DataV1R1 {
    pub seqno: u32,
    pub public_key: [u8; 32],
}

impl TryFrom<&Cell> for DataV1R1 {
    type Error = TonCellError;

    fn try_from(value: &Cell) -> Result<Self, Self::Error> {
        let mut parser = value.parser();
        let seqno = parser.load_u32(32)?;
        let mut public_key = [0u8; 32];
        parser.load_slice(&mut public_key)?;
        Ok(Self { seqno, public_key })
    }
}

impl TryInto<Cell> for DataV1R1 {
    type Error = TonCellError;

    fn try_into(self) -> Result<Cell, Self::Error> {
        CellBuilder::new()
            .store_u32(32, self.seqno)?
            .store_slice(&self.public_key)?
            .build()
    }
}

/// WalletVersion::V3R1 | WalletVersion::V3R2
pub struct DataV3R1 {
    pub seqno: u32,
    pub wallet_id: i32,
    pub public_key: [u8; 32],
}

impl TryFrom<&Cell> for DataV3R1 {
    type Error = TonCellError;

    fn try_from(value: &Cell) -> Result<Self, Self::Error> {
        let mut parser = value.parser();
        let seqno = parser.load_u32(32)?;
        let wallet_id = parser.load_i32(32)?;
        let mut public_key = [0u8; 32];
        parser.load_slice(&mut public_key)?;
        Ok(Self {
            seqno,
            wallet_id,
            public_key,
        })
    }
}

impl TryInto<Cell> for DataV3R1 {
    type Error = TonCellError;

    fn try_into(self) -> Result<Cell, Self::Error> {
        CellBuilder::new()
            .store_u32(32, self.seqno)?
            .store_i32(32, self.wallet_id)?
            .store_slice(&self.public_key)?
            .build()
    }
}

/// WalletVersion::V4R1 | WalletVersion::V4R2
pub struct DataV4R1 {
    pub seqno: u32,
    pub wallet_id: i32,
    pub public_key: [u8; 32],
}

impl TryFrom<&Cell> for DataV4R1 {
    type Error = TonCellError;

    fn try_from(value: &Cell) -> Result<Self, Self::Error> {
        let mut parser = value.parser();
        let seqno = parser.load_u32(32)?;
        let wallet_id = parser.load_i32(32)?;
        let mut public_key = [0u8; 32];
        parser.load_slice(&mut public_key)?;
        // TODO: handle plugin dict
        Ok(Self {
            seqno,
            wallet_id,
            public_key,
        })
    }
}

impl TryInto<Cell> for DataV4R1 {
    type Error = TonCellError;

    fn try_into(self) -> Result<Cell, Self::Error> {
        CellBuilder::new()
            .store_u32(32, self.seqno)?
            .store_i32(32, self.wallet_id)?
            .store_slice(&self.public_key)?
            // empty plugin dict
            .store_bit(false)?
            .build()
    }
}

/// WalletVersion::HighloadV2R2
pub struct DataHighloadV2R2 {
    pub wallet_id: i32,
    pub last_cleaned_time: u64,
    pub public_key: [u8; 32],
}

impl TryFrom<&Cell> for DataHighloadV2R2 {
    type Error = TonCellError;

    fn try_from(value: &Cell) -> Result<Self, Self::Error> {
        let mut parser = value.parser();
        let wallet_id = parser.load_i32(32)?;
        let last_cleaned_time = parser.load_u64(64)?;
        let mut public_key = [0u8; 32];
        parser.load_slice(&mut public_key)?;
        // TODO: handle queries dict
        Ok(Self {
            wallet_id,
            last_cleaned_time,
            public_key,
        })
    }
}

impl TryInto<Cell> for DataHighloadV2R2 {
    type Error = TonCellError;

    fn try_into(self) -> Result<Cell, Self::Error> {
        CellBuilder::new()
            .store_i32(32, self.wallet_id)?
            // TODO: not sure what goes into last_cleaned_time, so I set it to 0
            .store_u64(64, self.last_cleaned_time)?
            .store_slice(&self.public_key)?
            // empty plugin dict
            .store_bit(false)?
            .build()
    }
}
