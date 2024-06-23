use inbt::NbtTag;

#[derive(Debug)]
pub struct Section {
    // 4096 blocks
    blocks: Vec<String>,
    section_data: NbtTag,
}

impl Section {
    pub fn new(blocks: Vec<String>, section_data: NbtTag) -> Self {
        Self { blocks, section_data, }
    }

    /// Gets block relative to section origin
    pub fn get(&self, x: i32, y: i32, z: i32) -> Option<&String> {
        let range = 0..16;
        if !range.contains(&x) || !range.contains(&y) || !range.contains(&z) {
            return None;
        }
        let block_pos = y*16*16 + z*16 + x;
        Some(&self.blocks[block_pos as usize])
    }

    pub fn blocks(&self) -> &Vec<String> {
        &self.blocks
    }

    pub fn section_data(&self) -> &NbtTag {
        &self.section_data
    }
}