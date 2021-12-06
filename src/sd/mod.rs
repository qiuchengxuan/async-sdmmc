pub mod command;
pub mod data;
pub mod registers;
pub mod response;

pub const BLOCK_SIZE: usize = 512;

#[derive(Copy, Clone, Debug)]
pub enum Card {
    SDSC(u8),
    SDHC,
}

impl Card {
    pub fn high_capacity(self) -> bool {
        match self {
            Self::SDSC(_) => false,
            _ => true,
        }
    }
}
