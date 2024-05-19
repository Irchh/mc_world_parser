use thiserror::Error;

#[derive(Error, Debug)]
pub enum McaParseError {
    #[error("Faled parsing NBT: {0}")]
    NbtParseError(#[from] inbt::NbtParseError),
    #[error("Hit end of data")]
    EndOfData,
}