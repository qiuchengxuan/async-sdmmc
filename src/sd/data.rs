use core::convert::TryFrom;

#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub enum Error {
    NotToken,
    Generic,
    CC,
    CardECC,
    OutOfRange,
    CardLocked,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Token {
    Start = 0xFE,
    StartWriteMultipleBlock = 0xFC,
    Stop = 0xFD,
}

impl TryFrom<u8> for Token {
    type Error = Error;

    fn try_from(byte: u8) -> Result<Token, Error> {
        match (byte, byte ^ (byte & byte.wrapping_sub(1))) {
            (0xFE, _) => Ok(Token::Start),
            (0xFC, _) => Ok(Token::StartWriteMultipleBlock),
            (_, 0x10) => Err(Error::CardLocked),
            (_, 0x8) => Err(Error::OutOfRange),
            (_, 0x4) => Err(Error::CardECC),
            (_, 0x2) => Err(Error::CC),
            (_, 0x1) => Err(Error::Generic),
            (_, _) => Err(Error::NotToken),
        }
    }
}

pub enum Response {
    Accepted,
    CRCError,
    WriteError,
}

impl Response {
    pub fn try_from(byte: u8) -> Option<Self> {
        if byte & 0b10001 != 0x1 {
            return None;
        }
        let value = match (byte >> 1) & 0b111 {
            0b010 => Self::Accepted,
            0b101 => Self::CRCError,
            0b110 => Self::WriteError,
            _ => return None,
        };
        Some(value)
    }
}
