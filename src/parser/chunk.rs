use inbt::NbtTag;
use crate::Position;
use crate::parser::section::Section;

#[derive(Debug)]
pub struct Chunk {
    data_version: i32,
    position: Position,
    status: String,
    sections: Vec<Section>,
    chunk_data: NbtTag,
}

impl Chunk {
    pub fn new(data_version: i32, position: Position, status: String, sections: Vec<Section>, chunk_data: NbtTag) -> Self {
        Self {
            data_version,
            position,
            status,
            sections,
            chunk_data,
        }
    }
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

    pub fn data_version(&self) -> i32 {
        self.data_version
    }

    pub fn position(&self) -> &Position {
        &self.position
    }

    pub fn status(&self) -> &String {
        &self.status
    }

    pub fn sections(&self) -> &Vec<Section> {
        &self.sections
    }

    pub fn chunk_data(&self) -> &NbtTag {
        &self.chunk_data
    }
}