use anyhow::anyhow;

pub struct BinaryReader<'a> {
    data: &'a [u8],
    pos: usize,
}

// TODO: This should be converted to trait to allow generic implementation for std::io::Read
#[allow(dead_code)]
impl BinaryReader<'_> {
    pub fn new<'a>(data: &'a [u8]) -> BinaryReader<'a> {
        BinaryReader { data, pos: 0 }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn read_u8(&mut self) -> anyhow::Result<u8> {
        self.check_remaining(1)?;
        let res = self.data[self.pos];
        self.pos += 1;
        Ok(res)
    }

    pub fn read_u16_be(&mut self) -> anyhow::Result<u16> {
        self.check_remaining(2)?;
        let slice: [u8; 2] = self.data[self.pos..(self.pos + 2)].try_into()?;
        let res = u16::from_be_bytes(slice);
        self.pos += 2;
        Ok(res)
    }

    pub fn read_u16_le(&mut self) -> anyhow::Result<u16> {
        self.check_remaining(2)?;
        let slice: [u8; 2] = self.data[self.pos..(self.pos + 2)].try_into()?;
        let res = u16::from_le_bytes(slice);
        self.pos += 2;
        Ok(res)
    }

    pub fn read_u32_be(&mut self) -> anyhow::Result<u32> {
        self.check_remaining(4)?;
        let slice: [u8; 4] = self.data[self.pos..(self.pos + 4)].try_into()?;
        let res = u32::from_be_bytes(slice);
        self.pos += 4;
        Ok(res)
    }

    pub fn read_u32_le(&mut self) -> anyhow::Result<u32> {
        self.check_remaining(4)?;
        let slice: [u8; 4] = self.data[self.pos..(self.pos + 4)].try_into()?;
        let res = u32::from_le_bytes(slice);
        self.pos += 4;
        Ok(res)
    }

    pub fn read_bytes(&mut self, buf: &mut [u8]) -> anyhow::Result<()> {
        self.check_remaining(buf.len())?;
        let slice = &self.data[self.pos..(self.pos + buf.len())];
        buf.copy_from_slice(slice);
        self.pos += buf.len();
        Ok(())
    }

    pub fn read_var_size_be(&mut self, num_bytes: usize) -> anyhow::Result<usize> {
        self.check_remaining(num_bytes)?;
        let mut res: usize = 0;
        for _ in 0..num_bytes {
            res = res * 256 + (self.data[self.pos] as usize);
            self.pos += 1;
        }
        Ok(res)
    }

    fn check_remaining(&self, cnt: usize) -> anyhow::Result<()> {
        if self.data.len() - self.pos >= cnt {
            Ok(())
        } else {
            Err(anyhow!(
                "There's less than {} bytes remaining, size: {}, position: {}",
                cnt,
                self.data.len(),
                self.pos
            ))
        }
    }
}
