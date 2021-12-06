use bitfield::bitfield;

bitfield! {
    #[derive(Copy, Clone)]
    pub struct CSDv1(u128);
    pub version, _: 127, 126;
    pub max_read_data_block_length, _: 83, 80;
    pub device_size, _: 73, 62;
    pub device_size_multiplier, _: 49, 47;
}

impl CSDv1 {
    pub fn num_blocks(&self) -> usize {
        let multi = 1 << (self.device_size_multiplier() + 1);
        (self.device_size() as usize + 1) * multi * self.max_read_data_block_length() as usize / 512
    }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct CSDv2(u128);
    pub device_size, _: 69, 48;
}

impl CSDv2 {
    pub fn num_blocks(&self) -> usize {
        self.device_size() as usize + 1
    }
}

bitfield! {
    #[derive(Copy, Clone)]
    pub struct CSDv3(u128);
    pub device_size, _: 75, 48;
}

impl CSDv3 {
    pub fn num_blocks(&self) -> usize {
        self.device_size() as usize + 1
    }
}

#[derive(Copy, Clone)]
pub enum CSD {
    V1(CSDv1),
    V2(CSDv2),
    V3(CSDv3),
}

impl CSD {
    pub fn try_from(value: u128) -> Option<CSD> {
        let csd = match CSDv1(value).version() {
            0 => Self::V1(CSDv1(value)),
            1 => Self::V2(CSDv2(value)),
            2 => Self::V3(CSDv3(value)),
            _ => return None,
        };
        Some(csd)
    }

    pub fn num_blocks(&self) -> usize {
        match self {
            Self::V1(csd) => csd.num_blocks(),
            Self::V2(csd) => csd.num_blocks(),
            Self::V3(csd) => csd.num_blocks(),
        }
    }
}
