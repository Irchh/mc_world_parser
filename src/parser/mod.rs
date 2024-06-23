#![allow(dead_code)]

mod region;
mod chunk;
mod section;
pub mod parse_error;

use std::fmt::{Display, Formatter};
use std::iter::Peekable;
use std::slice::Iter;
use inbt::NbtTag;
use crate::parse_error::McaParseError;

#[derive(Debug)]
pub struct Position {
    x: i32,
    y: i32,
    z: i32,
}

impl Display for Position {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} / {} / {}", self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use crate::parser::region::Region;
    use super::*;

    #[test]
    fn parse_region() {
        let mut test_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_file.push("test_files/r.0.0.mca");
        let test_data = fs::read(test_file).expect("Failed to open test file");

        let region = Region::parse_region(test_data).unwrap();
        let chunk = &region.chunks()[0];
        let data_version = chunk.data_version();
        let pos = chunk.position();
        let status = chunk.status();

        eprintln!("Chunk data_version: {data_version}");
        eprintln!("Chunk XYZ: {}", pos);
        eprintln!("Chunk status: {status}");
        eprintln!("Chunk block: {:?}", region.get(24, 60, 15));
        eprintln!("Chunk finished: {}", chunk.is_finished());
    }
}
