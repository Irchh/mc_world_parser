use std::collections::BTreeMap;
use inbt::NbtTag;
use log::{debug, error};
use mc_datatypes::VarInt;
use crate::{Block, Position, McaParseError};

#[derive(Debug, Clone)]
pub struct Section {
    // 4096 blocks
    blocks: Vec<u16>, // Can hold numbers up to 64k, meanwhile each section can hold a max of 4k blocks
    palette: Vec<Block>,
}

pub trait BlockIDGetter {
    fn id_of(&self, block: &Block) -> i32;
}

impl Section {
    fn bits_needed_for_palette(palette_size: usize) -> usize {
        if palette_size == 1 {
            return 0;
        }
        let mut palette_bits = palette_size.checked_ilog2().unwrap_or(0) as usize;
        while usize::pow(2, palette_bits as u32) < palette_size {
            palette_bits += 1;
        }
        if palette_bits < 4 {
            palette_bits = 4;
        }
        if palette_bits > 8 {
            palette_bits = 15;
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
                blocks: vec![0; 4096],
                palette: vec![Block::new(palette[0].get_string("Name")?)],
            });
        }
        let block_data = block_states.get_long_array("data")?;

        // Bits needed to store the index into palette list, minimum 4 bits.
        let palette_bits = Self::bits_needed_for_palette(palette.len());
        // Calculate the palette mask
        let palette_mask = Self::palette_mask(palette_bits);
        let palette_entries_per_long = 64/palette_bits;

        let mut blocks = vec![Block::new("".to_string()); 4096];
        for y in 0..16 {
            for z in 0..16 {
                for x in 0..16 {
                    let block_pos = y*16*16 + z*16 + x;
                    let block_data_index = block_pos/palette_entries_per_long;
                    let block_data_sub_index = block_pos%palette_entries_per_long;
                    let mask_shift = palette_bits*block_data_sub_index;
                    let palette_index = (block_data[block_data_index] as u64 & (palette_mask<<mask_shift))>>mask_shift;
                    if palette_index as usize >= palette.len() {
                        // Will panic after this, just debug info for now
                        error!("palette_bits: {}", palette_bits);
                        error!("palette_mask: {:0b}", palette_mask);
                        error!("block_data: {:064b}", block_data[block_data_index]);
                        error!("block_data_index: {}", block_data_sub_index);
                        error!("palette_index: {palette_index}");
                    }
                    let block = &palette[palette_index as usize];
                    let block_name = block.get_string("Name").unwrap();
                    blocks[block_pos] = Block::new(block_name);
                    //blocks.push(Block::new(block_name));
                }
            }
        }

        let mut palette = vec![];
        let mut palette_indexes = vec![];
        for block in blocks {
            if !palette.contains(&block) {
                palette.push(block.clone());
            }
            let index = palette.iter().enumerate().find(|b| b.1.eq(&block)).unwrap().0;
            palette_indexes.push(index as u16);
        }

        Ok(Section {
            blocks: palette_indexes,
            palette,
        })
    }

    /// Takes a function to map identifiers to numbers, e.g. minecraft:air -> 0
    pub fn network_data(&self, id_getter: &Box<dyn BlockIDGetter>) -> Vec<u8> {
        let mut network_data = vec![];
        let mut palette: Vec<Block> = vec![];
        let mut block_count = 0;
        for block_indexes in &self.blocks {
            let block = &self.palette[*block_indexes as usize];
            if !palette.contains(block) {
                palette.push(block.clone());
            }
            block_count += (!block.identifier.eq("minecraft:air")) as u16;
        }

        let mut bits_per_entry = Self::bits_needed_for_palette(palette.len());

        // Block count as short
        network_data.append(&mut block_count.to_be_bytes().to_vec());

        network_data.push(bits_per_entry as u8);

        if bits_per_entry == 0 {
            network_data.append(&mut VarInt::new(id_getter.id_of(&palette[0])).bytes);
            network_data.push(0);
        } else if (4..9).contains(&bits_per_entry) {
            network_data.append(&mut VarInt::new(palette.len() as i32).bytes);
            network_data.append(&mut palette.iter().flat_map(|s| VarInt::new(id_getter.id_of(s)).bytes).collect::<Vec<u8>>());

            let entries_per_long = 64/bits_per_entry;

            let mut longs = vec![];
            for blocks in self.blocks.chunks(entries_per_long) {
                let mut long = 0;
                for (i, block_index) in blocks.iter().enumerate() {
                    let block = &self.palette[*block_index as usize];
                    let mut palette_index = None;
                    for (i, palette_entry) in palette.iter().enumerate() {
                        if palette_entry.eq(block) {
                            palette_index = Some(i);
                            break;
                        }
                    }
                    long |= palette_index.unwrap()<<(i*bits_per_entry)
                }
                longs.push(long);
            }
            network_data.append(&mut VarInt::new(longs.len() as i32).bytes);
            network_data.append(&mut longs.iter().flat_map(|l| l.to_be_bytes().to_vec()).collect());
        } else {
            let entries_per_long = 64/bits_per_entry;

            let mut longs = vec![];
            for blocks in self.blocks.chunks(entries_per_long) {
                let mut long = 0;
                for (i, block_index) in blocks.iter().enumerate() {
                    let block = &self.palette[*block_index as usize];
                    let block_id = id_getter.id_of(block) as u64;
                    long |= block_id<<(i*bits_per_entry)
                }
                longs.push(long);
            }
            network_data.append(&mut VarInt::new(longs.len() as i32).bytes);
            network_data.append(&mut longs.iter().flat_map(|l| l.to_be_bytes().to_vec()).collect());
        }

        // Fake biome info
        network_data.push(0); // Only a single biome so no bits per entry
        network_data.append(&mut VarInt::new(8).bytes); // Which biome? biome nr. 8
        network_data.push(0); // Data array is not included, but we still need to have the length
        network_data
    }

    /// Gets block relative to section origin
    pub fn get(&self, pos: Position) -> Block {
        self.palette[self.blocks[pos.block_index_in_section()] as usize].clone()
    }
}