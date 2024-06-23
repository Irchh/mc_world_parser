use inbt::NbtTag;
use crate::McaParseError;

#[derive(Debug)]
pub struct Section {
    // 4096 blocks
    blocks: Vec<String>,
    section_data: NbtTag,
}

impl Section {
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