#![allow(dead_code)]

pub mod region;
pub mod chunk;
pub mod section;
pub mod level;
pub mod parse_error;

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use inbt::{NbtParseError, NbtTag};

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Eq for Position {}

impl Ord for Position {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.y > other.y {
            Ordering::Greater
        } else if self.y < other.y {
            Ordering::Less
        } else if self.x > other.x {
            Ordering::Greater
        } else if self.x < other.x {
            Ordering::Less
        } else if self.z > other.z {
            Ordering::Greater
        } else if self.z < other.z {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
}

impl Position {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn region_in_world(&self) -> Self {
        Self::new(self.x>>9, 0, self.z>>9)
    }

    pub fn chunk_in_region(&self) -> Self {
        // If negative then we subtract an extra 1
        let x = self.x/16 - (self.x < 0) as i32;
        let z = self.z/16 - (self.z < 0) as i32;
        Self::new(x, 0, z)
    }

    pub fn section_index_in_chunk(&self) -> Option<u32> {
        let mut y = self.y;
        if !(-64..320).contains(&y) {
            return None;
        }
        y += 64;
        let section = y/16;
        Some(section as u32)
    }

    pub fn block_in_section(&self) -> Self {
        let x = self.x.rem_euclid(16);
        let y = self.y.rem_euclid(16);
        let z = self.z.rem_euclid(16);
        Self::new(x, y, z)
    }

    pub fn block_index_in_section(&self) -> usize {
        let pos = self.block_in_section();
        let block_pos = pos.y*16*16 + pos.z*16 + pos.x;
        block_pos as usize
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} / {} / {}", self.x, self.y, self.z)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    identifier: String,
    properties: BTreeMap<String, String>
}

impl Block {
    pub fn new(nbt_data: &NbtTag) -> Result<Self, NbtParseError> {
        let identifier = nbt_data.get_string("Name")?;
        let nbt_properties = nbt_data.get_compound("Properties").unwrap_or(vec![]);
        let mut properties = BTreeMap::new();

        for property in nbt_properties {
            match property {
                NbtTag::String(name, value) => {
                    properties.insert(name, value);
                }
                _ => {}
            }
        }
        Ok(Self {
            identifier,
            properties,
        })
    }

    pub fn default() -> Self {
        Self {
            identifier: "minecraft:air".to_string(),
            properties: Default::default(),
        }
    }

    pub fn identifier(&self) -> &String {
        &self.identifier
    }

    pub fn properties(&self) -> &BTreeMap<String, String> {
        &self.properties
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use crate::parser::region::Region;
    use crate::{Block, Position, World};

    #[test]
    fn position_conversion() {
        assert_eq!(Position { x: 0, y: 0, z: 0 }.region_in_world(), Position::new(0, 0, 0));
        assert_eq!(Position { x: -512, y: 0, z: 0 }.region_in_world(), Position::new(-1, 0, 0));
        assert_eq!(Position { x: -1, y: 0, z: 0 }.region_in_world(), Position::new(-1, 0, 0));
    }

    #[test]
    fn parse_world() {
        let mut world = World::load("test_files/world").unwrap();
        eprintln!("World: {:?}", world);
        eprintln!("World: {:?}", world.get_block(Position::new(-1, 83, 1)));

        assert_eq!(world.get_block(Position::new(24, 60, 15)), Some(Block { identifier: "minecraft:water".to_string(), properties: BTreeMap::from([("level".to_string(),  "0".to_string())]) }));

        let mut test_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_file.push("test_files/r.0.0.mca");
        let test_data = fs::read(test_file).expect("Failed to open test file");

        let region = Region::parse_region(test_data).unwrap();
        let chunk = &region.chunks()[0];
        let data_version = chunk.data_version();
        let pos = chunk.chunk_pos();
        let status = chunk.status();

        eprintln!("Chunk data_version: {data_version}");
        eprintln!("Chunk XYZ: {}", pos);
        eprintln!("Chunk status: {status}");
        eprintln!("Chunk block: {:?}", region.get(Position::new(24, 60, 15)));
        eprintln!("Chunk finished: {}", chunk.is_finished());
    }
}
