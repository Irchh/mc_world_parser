use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::fs::DirEntry;
use std::path::{Path, PathBuf};
use log::{debug, error};
use crate::{Block, McaParseError, Position};
use crate::parser::chunk::Chunk;
use crate::parser::level::Level;
use crate::parser::region::Region;

#[derive(Debug)]
pub struct World {
    level_dir_entries: Vec<DirEntry>,
    level: Level,

    region_path: PathBuf,
    loaded_regions: BTreeMap<Position, Region>
}

impl World {
    /// Loads a Minecraft world from its path.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, McaParseError> {
        let world_dir = fs::read_dir(path)?.filter_map(|e| e.ok()).collect::<Vec<DirEntry>>();
        let level_dat = world_dir.iter().find(|e| e.file_name() == OsString::from("level.dat")).ok_or(McaParseError::InvalidWorld)?;
        let level_data = fs::read(level_dat.path())?;
        let level = Level::parse_level(level_data)?;

        let region_path = world_dir.iter().find(|e| e.file_name() == OsString::from("region")).ok_or(McaParseError::InvalidWorld)?.path();
        Ok(Self {
            level_dir_entries: world_dir,
            level,
            region_path,
            loaded_regions: BTreeMap::new(),
        })
    }

    pub fn get_block(&mut self, pos: Position) -> Option<Block> {
        // I would like to extract the region getting to its own function, but lifetime shenanigans causes trouble
        let region = if let Some(region) = self.loaded_regions.get(&pos.region_in_world()) {
            region
        } else {
            self.load_region(pos.region_in_world())?;
            self.loaded_regions.get(&pos.region_in_world())?
        };
        region.get(pos).cloned()
    }

    pub fn get_chunk(&mut self, pos: Position) -> Option<Chunk> {
        let region = if let Some(region) = self.loaded_regions.get(&pos.region_in_world()) {
            region
        } else {
            self.load_region(pos.region_in_world())?;
            self.loaded_regions.get(&pos.region_in_world())?
        };
        region.get_chunk(pos).cloned()
    }

    fn load_region(&mut self, pos: Position) -> Option<()> {
        debug!("Loading region: r.{}.{}.mca", pos.x, pos.z);
        let region_data = fs::read(self.region_path.as_path().join(format!("r.{}.{}.mca", pos.x, pos.z))).ok()?;
        let region = Region::parse_region(region_data);
        if region.is_err() {
            error!("Error parsing region: {}", region.err().unwrap());
            return None;
        }
        self.loaded_regions.insert(pos, region.ok()?);
        Some(())
    }
}