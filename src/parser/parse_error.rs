use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum McaParseError {
    #[error("Error loading world: {0}")]
    WorldLoadError(#[from] io::Error),
    #[error("Faled parsing NBT: {0}")]
    NbtParseError(#[from] inbt::NbtParseError),
    #[error("Hit end of data")]
    EndOfData,
}