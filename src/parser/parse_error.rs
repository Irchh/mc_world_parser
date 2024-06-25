use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum McaParseError {
    #[error("Error loading world: {0}")]
    WorldLoadError(#[from] io::Error),
    #[error("Failed parsing NBT: {0}")]
    NbtParseError(#[from] inbt::NbtParseError),
    #[error("Specified world directory is not a valid minecraft world")]
    InvalidWorld,
    #[error("Hit end of data")]
    EndOfData,
}