#![allow(dead_code)]

use std::fmt::{Display, Formatter};
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

#[derive(Debug)]
pub struct Section {
    // 4096 blocks
    blocks: Vec<String>,
    section_data: NbtTag,
}

impl Section {
    /// Gets block relative to section origin
    pub fn get(&self, x: i32, y: i32, z: i32) -> Option<&String> {
        let range = 0..16;
        if !range.contains(&x) || !range.contains(&y) || !range.contains(&z) {
            return None;
        }
        let block_pos = y*16*16 + z*16 + x;
        Some(&self.blocks[block_pos as usize])
    }
}

#[derive(Debug)]
pub struct Chunk {
    data_version: i32,
    position: Position,
    status: String,
    sections: Vec<Section>,
    chunk_data: NbtTag,
}

impl Chunk {
    /// Gets block relative to chunk origin
    pub fn get(&self, x: i32, mut y: i32, z: i32) -> Option<&String> {
        if !(-64..320).contains(&y) {
            return None;
        }
        y += 64;
        let section = y/16;
        self.sections[section as usize].get(x, y%16, z)
    }

    pub fn is_finished(&self) -> bool {
        &*self.status == "minecraft:full"
    }
}

#[derive(Debug)]
pub struct Region {
    chunk_location_offsets: Vec<ChunkLocation>,
    chunk_timestamps: Vec<ChunkTimestamp>,
    chunks: Vec<Chunk>,
}

impl Region {
    /// Gets block relative to region origin
    pub fn get(&self, x: i32, y: i32, z: i32) -> Option<&String> {
        let chunk = self.get_chunk(x/16, z/16)?;
        chunk.get(x%16, y, z%16)
    }

    pub fn get_chunk(&self, x: i32, z: i32) -> Option<&Chunk> {
        for chunk in &self.chunks {
            if chunk.position.x == x && chunk.position.z == z {
                return Some(chunk);
            }
        }
        None
    }

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
    fn next_chunk(iterable: &mut Peekable<Iter<u8>>) -> Result<Chunk, McaParseError> {
        let length = Self::next_int(iterable)?;
        // 1 - GZip (usually not used)
        // 2 - Zlib
        // 3 - Uncompressed (usually not used)
        let compression_type = Self::next_byte(iterable)?;
        let raw_data = iterable.take((length - 1) as usize).map(|n| *n).collect::<Vec<u8>>();
        if raw_data.len() < (length - 1) as usize {
            return Err(McaParseError::EndOfData);
        }
        // TODO: convert to ParseError
        let parser_result = match compression_type {
            1 => inbt::nbt_parser::parse_gzip(raw_data),
            2 => inbt::nbt_parser::parse_zlib(raw_data),
            3 => Ok(inbt::nbt_parser::parse_binary(raw_data)),
            _ => unimplemented!()
        }.unwrap();
        let sections = Self::parse_sections(parser_result.get_list("sections")?)?;
        Ok(Chunk {
            data_version: parser_result.get_int("DataVersion")?,
            position: Position {
                x: parser_result.get_int("xPos")?,
                y: parser_result.get_int("yPos")?,
                z: parser_result.get_int("zPos")?,
            },
            status: parser_result.get_string("Status")?,
            sections,
            chunk_data: parser_result,
        })
    }

    pub fn parse_sections(data: Vec<NbtTag>) -> Result<Vec<Section>, McaParseError> {
        let mut sections = vec![];
        for tag in data {
            sections.push(Self::parse_section(tag)?)
        }
        Ok(sections)
    }

    pub fn parse_section(tag: NbtTag) -> Result<Section, McaParseError> {
        let block_states = tag.get("block_states")?;
        let palette = block_states.get_list("palette")?;
        if palette.len() == 1 {
            return Ok(Section {
                blocks: vec![palette[0].get_string("Name")?; 4096],
                section_data: tag,
            });
        }
        let block_data = block_states.get_long_array("data")?;

        // Bits needed to store the index into palette list, minimum 4 bits.
        let mut palette_bits = palette.len().checked_ilog2().unwrap_or(0) as usize;
        if usize::pow(2, palette_bits as u32) < palette.len() {
            palette_bits += 1;
        }
        if palette_bits < 4 {
            palette_bits = 4;
        }
        // Calculate the palette mask
        let mut palette_mask = 0b0;
        for _ in 0..palette_bits {
            // There's probably a better way of doing this..
            palette_mask <<= 1;
            palette_mask |= 1;
        }
        let palette_entries_per_long = 64/palette_bits;
        let padding = 64%palette_bits;

        let mut blocks = vec![];
        for y in 0..16 {
            for z in 0..16 {
                for x in 0..16 {
                    let block_pos = y*16*16 + z*16 + x;
                    let block_data_index = block_pos/palette_entries_per_long;
                    let block_data_sub_index = block_pos%palette_entries_per_long + 1;
                    let mask_shift = (64-padding)-palette_bits*block_data_sub_index;
                    let palette_index = (block_data[block_data_index] as u64 & (palette_mask<<mask_shift))>>mask_shift;
                    if palette_index as usize >= palette.len() {
                        // Will panic after this, just debug info for now
                        eprintln!("palette_bits: {}", palette_bits);
                        eprintln!("palette_mask: {:0b}", palette_mask);
                        eprintln!("block_data: {:064b}", block_data[block_data_index]);
                        eprintln!("block_data_index: {}", block_data_sub_index-1);
                        eprintln!("palette_index: {palette_index}");
                    }
                    let block = &palette[palette_index as usize];
                    let block_name = block.get_string("Name").unwrap();
                    blocks.push(block_name);
                }
            }
        }

        Ok(Section {
            blocks,
            section_data: tag,
        })
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
        let chunk = &region.chunks[0];
        let data_version = chunk.data_version;
        let pos = &chunk.position;
        let status = &chunk.status;

        eprintln!("Chunk data_version: {data_version}");
        eprintln!("Chunk XYZ: {}", pos);
        eprintln!("Chunk status: {status}");
        eprintln!("Chunk block: {:?}", region.get(24, 60, 15));
        eprintln!("Chunk finished: {}", chunk.is_finished());
    }
}