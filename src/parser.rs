#![allow(dead_code)]

use std::iter::Peekable;
use std::slice::Iter;
use inbt::NbtTag;
use crate::parse_error::McaParseError;

#[derive(Debug)]
pub struct ChunkLocation {
    /// Offset in 4KiB sectors from the start of the file.
    ///
    /// Note: If both this and `sectors` are 0 the chunk is not present.
    offset: usize,
    /// How many sectors (4KiB size) the chunk consists of. Max of 255 sectors.
    ///
    /// Note: If both this and `offset` are 0 the chunk is not present.
    sectors: usize,
}

#[derive(Debug)]
pub struct ChunkTimestamp {
    /// Represents the last modification time of a chunk in epoch seconds.
    modified_seconds: u32,
}

#[derive(Debug)]
pub struct Chunk {
    chunk_data: Vec<NbtTag>
}

#[derive(Debug)]
pub struct Region {
    chunk_location_offsets: Vec<ChunkLocation>,
    chunk_timestamps: Vec<ChunkTimestamp>,
    chunks: Vec<NbtTag>,
}

impl Region {
    fn next(iterable: &mut Peekable<Iter<u8>>) -> Result<u8, McaParseError> {
        iterable.next().map(|n| *n).ok_or(McaParseError::EndOfData)
    }

    fn next_byte(iterable: &mut Peekable<Iter<u8>>) -> Result<i8, McaParseError> {
        Ok(i8::from_be_bytes([Self::next(iterable)?]))
    }

    fn next_short(iterable: &mut Peekable<Iter<u8>>) -> Result<i16, McaParseError> {
        Ok(i16::from_be_bytes([Self::next(iterable)?, Self::next(iterable)?]))
    }

    fn next_int(iterable: &mut Peekable<Iter<u8>>) -> Result<i32, McaParseError> {
        Ok(i32::from_be_bytes([
            Self::next(iterable)?, Self::next(iterable)?, Self::next(iterable)?, Self::next(iterable)?
        ]))
    }

    fn next_long(iterable: &mut Peekable<Iter<u8>>) -> Result<i64, McaParseError> {
        Ok(i64::from_be_bytes([
            Self::next(iterable)?, Self::next(iterable)?, Self::next(iterable)?, Self::next(iterable)?,
            Self::next(iterable)?, Self::next(iterable)?, Self::next(iterable)?, Self::next(iterable)?
        ]))
    }

    fn next_chunk_location(iterable: &mut Peekable<Iter<u8>>) -> Result<ChunkLocation, McaParseError> {
        Ok(ChunkLocation {
            offset: u32::from_be_bytes([0, Self::next(iterable)?, Self::next(iterable)?, Self::next(iterable)?]) as usize,
            sectors: Self::next(iterable)? as usize,
        })
    }

    fn next_chunk_timestamp(iterable: &mut Peekable<Iter<u8>>) -> Result<ChunkTimestamp, McaParseError> {
        Ok(ChunkTimestamp {
            modified_seconds: u32::from_be_bytes([ Self::next(iterable)?, Self::next(iterable)?, Self::next(iterable)?, Self::next(iterable)? ])}
        )
    }
    fn next_chunk(iterable: &mut Peekable<Iter<u8>>) -> Result<NbtTag, McaParseError> {
        let length = Self::next_int(iterable)?;
        // 1 - GZip (usually not used)
        // 2 - Zlib
        // 3 - Uncompressed (usually not used)
        let compression_type = Self::next_byte(iterable)?;
        let raw_data = iterable.take((length - 1) as usize).map(|n| *n).collect::<Vec<u8>>();
        if raw_data.len() < (length - 1) as usize {
            return Err(McaParseError::EndOfData);
        }
        let parser_result = match compression_type {
            1 => inbt::nbt_parser::parse_gzip(raw_data),
            2 => inbt::nbt_parser::parse_zlib(raw_data),
            3 => Ok(inbt::nbt_parser::parse_binary(raw_data)),
            _ => unimplemented!()
        };
        // TODO: convert to ParseError
        Ok(parser_result.unwrap())
    }

    pub fn parse_region(region_data: Vec<u8>) -> Result<Region, McaParseError> {
        if region_data.len() < 0x2000 {
            return Err(McaParseError::EndOfData);
        }
        let mut data = region_data[0..8192].iter().peekable();
        let mut chunk_locations = vec![];
        let mut chunk_timestamps = vec![];
        for _ in 0..1024 {
            chunk_locations.push(Self::next_chunk_location(&mut data)?);
        }
        for _ in 0..1024 {
            chunk_timestamps.push(Self::next_chunk_timestamp(&mut data)?)
        }

        let mut chunks = vec![];
        for loc in &chunk_locations {
            if loc.offset != 0 && loc.sectors != 0 {
                chunks.push(Self::next_chunk(&mut region_data[(loc.offset*4096)..(loc.offset*4096+loc.sectors*4096)].iter().peekable())?);
            } else {
                chunks.push(NbtTag::Compound("".to_string(), vec![]));
            }
        }
        Ok(Region {
            chunk_location_offsets: chunk_locations,
            chunk_timestamps,
            chunks: chunks,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use super::*;

    #[test]
    fn parse_region() {
        let mut test_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_file.push("test_files/r.0.0.mca");
        let test_data = fs::read(test_file).expect("Failed to open test file");

        let region = Region::parse_region(test_data).unwrap();
        let data_version = region.chunks[0].get_int("DataVersion").unwrap();

        eprintln!("Chunk data_version: {data_version}")
    }
}