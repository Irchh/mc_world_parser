use inbt::NbtTag;
use crate::McaParseError;

#[derive(Debug)]
pub struct Level {
    nbt: NbtTag
}

impl Level {
    pub fn parse_level(level_data: Vec<u8>) -> Result<Self, McaParseError> {
        let nbt = inbt::nbt_parser::parse_gzip(level_data)?;
        Ok(Self { nbt })
    }
}