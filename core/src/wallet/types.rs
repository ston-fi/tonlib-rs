use crate::cell::{Cell, CellBuilder, TonCellError};

/// WalletVersion::V1R1 | WalletVersion::V1R2 | WalletVersion::V1R3 | WalletVersion::V2R1 | WalletVersion::V2R2
pub struct WalletDataV1V2 {
    pub seqno: u32,
    pub public_key: [u8; 32],
}

impl TryFrom<Cell> for WalletDataV1V2 {
    type Error = TonCellError;

    fn try_from(value: Cell) -> Result<Self, Self::Error> {
        let mut parser = value.parser();
        let seqno = parser.load_u32(32)?;
        let mut public_key = [0u8; 32];
        parser.load_slice(&mut public_key)?;
        Ok(Self { seqno, public_key })
    }
}

impl TryFrom<WalletDataV1V2> for Cell {
    type Error = TonCellError;

    fn try_from(value: WalletDataV1V2) -> Result<Self, Self::Error> {
        CellBuilder::new()
            .store_u32(32, value.seqno)?
            .store_slice(&value.public_key)?
            .build()
    }
}

/// WalletVersion::V3R1 | WalletVersion::V3R2
pub struct WalletDataV3 {
    pub seqno: u32,
    pub wallet_id: i32,
    pub public_key: [u8; 32],
}

impl TryFrom<Cell> for WalletDataV3 {
    type Error = TonCellError;

    fn try_from(value: Cell) -> Result<Self, Self::Error> {
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

impl TryFrom<WalletDataV3> for Cell {
    type Error = TonCellError;

    fn try_from(value: WalletDataV3) -> Result<Self, Self::Error> {
        CellBuilder::new()
            .store_u32(32, value.seqno)?
            .store_i32(32, value.wallet_id)?
            .store_slice(&value.public_key)?
            .build()
    }
}

/// WalletVersion::V4R1 | WalletVersion::V4R2
pub struct WalletDataV4 {
    pub seqno: u32,
    pub wallet_id: i32,
    pub public_key: [u8; 32],
}

impl TryFrom<Cell> for WalletDataV4 {
    type Error = TonCellError;

    fn try_from(value: Cell) -> Result<Self, Self::Error> {
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

impl TryFrom<WalletDataV4> for Cell {
    type Error = TonCellError;

    fn try_from(value: WalletDataV4) -> Result<Self, Self::Error> {
        CellBuilder::new()
            .store_u32(32, value.seqno)?
            .store_i32(32, value.wallet_id)?
            .store_slice(&value.public_key)?
            // empty plugin dict
            .store_bit(false)?
            .build()
    }
}

/// WalletVersion::HighloadV2R2
pub struct WalletDataHighloadV2R2 {
    pub wallet_id: i32,
    pub last_cleaned_time: u64,
    pub public_key: [u8; 32],
}

impl TryFrom<Cell> for WalletDataHighloadV2R2 {
    type Error = TonCellError;

    fn try_from(value: Cell) -> Result<Self, Self::Error> {
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

impl TryFrom<WalletDataHighloadV2R2> for Cell {
    type Error = TonCellError;

    fn try_from(value: WalletDataHighloadV2R2) -> Result<Self, Self::Error> {
        CellBuilder::new()
            .store_i32(32, value.wallet_id)?
            // TODO: not sure what goes into last_cleaned_time, so I set it to 0
            .store_u64(64, value.last_cleaned_time)?
            .store_slice(&value.public_key)?
            // empty plugin dict
            .store_bit(false)?
            .build()
    }
}
