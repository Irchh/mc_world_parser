use std::collections::{BTreeMap, BTreeSet, HashMap};
use inbt::NbtTag;
use log::{error, trace};
use mc_datatypes::{VarInt, VarLong};
use crate::{Block, Position, McaParseError};

#[derive(Debug, Clone)]
pub struct Section {
    // 4096 blocks
    blocks: Vec<Block>,
    section_data: NbtTag,
}

impl Section {
    fn bits_needed_for_palette(palette_size: usize) -> usize {
        if palette_size == 1 {
            return 0;
        }
        let mut palette_bits = palette_size.checked_ilog2().unwrap_or(0) as usize;
        if usize::pow(2, palette_bits as u32) < palette_size {
            palette_bits += 1;
        }
        if palette_bits < 4 {
            palette_bits = 4;
        }
        palette_bits
    }

    fn palette_mask(palette_bits: usize) -> u64 {
        if palette_bits > 64 {
            panic!("Palette bits out of range!")
        }
        let mut palette_mask = 0b0;
        for _ in 0..palette_bits {
            // There's probably a better way of doing this.
            palette_mask <<= 1;
            palette_mask |= 1;
        }
        palette_mask
    }

    pub fn parse_section(tag: NbtTag) -> Result<Section, McaParseError> {
        let block_states = tag.get("block_states")?;
        let palette = block_states.get_list("palette")?;
        if palette.len() == 1 {
            return Ok(Section {
                blocks: vec![Block::new(palette[0].get_string("Name")?); 4096],
                section_data: tag,
            });
        }
        let block_data = block_states.get_long_array("data")?;

        // Bits needed to store the index into palette list, minimum 4 bits.
        let mut palette_bits = Self::bits_needed_for_palette(palette.len());
        // Calculate the palette mask
        let mut palette_mask = Self::palette_mask(palette_bits);
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
                        error!("palette_bits: {}", palette_bits);
                        error!("palette_mask: {:0b}", palette_mask);
                        error!("block_data: {:064b}", block_data[block_data_index]);
                        error!("block_data_index: {}", block_data_sub_index-1);
                        error!("palette_index: {palette_index}");
                    }
                    let block = &palette[palette_index as usize];
                    let block_name = block.get_string("Name").unwrap();
                    blocks.push(Block::new(block_name));
                }
            }
        }

        Ok(Section {
            blocks,
            section_data: tag,
        })
    }

    /// Takes a function to map identifiers to numbers, e.g. minecraft:air -> 0
    pub fn network_data(&self, f: fn(&String) -> i32) -> Vec<u8> {
        let palette = self.blocks.iter().cloned()
            .map(|b| b.identifier)
            .collect::<Vec<_>>();

        let block_count = self.blocks.iter().filter(|b| !b.identifier.eq("minecraft:air")).count() as i32;
        let mut block_indexes = self.blocks.iter().map(|block| {
            let (palette_index, _) = palette.iter().enumerate().find(|(i, s)| (*s).eq(&block.identifier)).unwrap();
            palette_index
        });

        let mut network_data = vec![];
        // Block count
        network_data.append(&mut VarInt::new(block_count).bytes);

        let palette_bits = Self::bits_needed_for_palette(palette.len());
        let palette_mask = Self::palette_mask(palette_bits);
        let padding = 64%palette_bits;
        let entries_per_long = 64/palette_bits;
        let mut palette_longs = vec![];

        if palette_bits == 0 {
            network_data.push(0);
            network_data.append(&mut VarInt::new(f(&palette[0])).bytes);
            network_data.push(0);
        } else if (4..=8).contains(&palette_bits) {
            for (i, block_index) in block_indexes.enumerate() {
                if i%entries_per_long == 0 {
                    palette_longs.push(0);
                }
                let current_long = palette_longs.last_mut().unwrap();
                *current_long |= ((block_index as u64)&palette_mask)<<(64-padding-palette_bits*(i%entries_per_long+1));
            }

            // Block states
            network_data.push(palette_bits as u8);
            // Indirect
            network_data.append(&mut VarInt::new(palette.len() as i32).bytes);
            network_data.append(&mut palette.iter().flat_map(|s| VarInt::new(f(s)).bytes).collect::<Vec<u8>>());
            // Data array
            network_data.append(&mut palette_longs.iter().flat_map(|i| u64::to_be_bytes(*i)).collect::<Vec<u8>>());
        } else {
            for (i, block) in self.blocks.iter().enumerate() {
                if i%entries_per_long == 0 {
                    palette_longs.push(0);
                }
                let current_long = palette_longs.last_mut().unwrap();
                //trace!("padding: {padding}, palette_bits: {palette_bits}, entries_per_long: {entries_per_long}");
                *current_long |= ((f(&block.identifier) as u64)&palette_mask)<<(64-padding-palette_bits*(i%entries_per_long+1));
            }

            network_data.push(palette_bits as u8);
            network_data.append(&mut palette_longs.iter().flat_map(|i| u64::to_be_bytes(*i)).collect::<Vec<u8>>());
        }

        // Fake biome info
        network_data.push(0); // Only a single biome so no bits per entry
        network_data.append(&mut VarInt::new(0).bytes); // Which biome? biome nr. 0
        network_data.push(0); // Data array is not included but we still need to have the length
        network_data
    }

    /// Gets block relative to section origin
    pub fn get(&self, pos: Position) -> &Block {
        &self.blocks[pos.block_index_in_section()]
    }

    pub fn blocks(&self) -> &Vec<Block> {
        &self.blocks
    }

    pub fn section_data(&self) -> &NbtTag {
        &self.section_data
    }
}