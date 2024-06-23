use inbt::NbtTag;
use crate::{Block, Position};
use crate::parser::section::Section;

#[derive(Debug)]
pub struct Chunk {
    data_version: i32,
    chunk_pos: Position,
    status: String,
    sections: Vec<Section>,
    chunk_data: NbtTag,
}

impl Chunk {
    pub fn new(data_version: i32, chunk_pos: Position, status: String, sections: Vec<Section>, chunk_data: NbtTag) -> Self {
        Self {
            data_version,
            chunk_pos,
            status,
            sections,
            chunk_data,
        }
    }
    /// Gets block relative to chunk origin
    pub fn get(&self, pos: Position) -> Option<&Block> {
        let section = pos.section_index_in_chunk();
        if section.is_none() {
            eprintln!("Warning: section index out of bounds (Original Y: {})", pos.y);
        }
        Some(self.sections[section? as usize].get(pos))
    }

    pub fn is_finished(&self) -> bool {
        &*self.status == "minecraft:full"
    }

    pub fn data_version(&self) -> i32 {
        self.data_version
    }

    pub fn chunk_pos(&self) -> &Position {
        &self.chunk_pos
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