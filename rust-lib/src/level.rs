use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::mem;

const MAX_NUM_ENEMIES: usize = 50;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImportStage {
    pub name: [u8; 100],
    pub number: i32,
    pub array: [[i32; 30]; 256],
    pub tile_type: i32,
    pub background_colour: i32,
    pub start_position_x: [i32; 4],
    pub start_position_y: [i32; 4],
    pub warp_to_stage: i32,
    pub platform_in_use: i32,
    pub platform_x: i32,
    pub platform_y: i32,
    pub platform_type: i32,
    pub platform_direction: i32,
    pub platform_path_length: i32,
    pub platform_move_pos: i32,
    pub enemy_type: [i32; MAX_NUM_ENEMIES],
    pub enemy_in_use: [i32; MAX_NUM_ENEMIES],
    pub enemy_direction: [i32; MAX_NUM_ENEMIES],
    pub enemy_pos_x: [i32; MAX_NUM_ENEMIES],
    pub enemy_pos_y: [i32; MAX_NUM_ENEMIES],
    pub substage_array: [[i32; 30]; 40],
    pub substage_start_position_x: i32,
    pub substage_start_position_y: i32,
    pub substage_platform_in_use: i32,
    pub substage_platform_x: i32,
    pub substage_platform_y: i32,
    pub substage_platform_type: i32,
    pub substage_platform_direction: i32,
    pub substage_platform_path_length: i32,
    pub substage_platform_move_pos: i32,
}

impl ImportStage {
    pub fn new() -> Self {
        unsafe { mem::zeroed() }
    }
}

pub fn load_stage(file_path: &str, stage_index: usize) -> io::Result<ImportStage> {
    let mut file = File::open(file_path)?;
    let struct_size = mem::size_of::<ImportStage>();
    file.seek(SeekFrom::Start((stage_index * struct_size) as u64))?;
    let mut stage = ImportStage::new();
    let buffer = unsafe {
        std::slice::from_raw_parts_mut(
            &mut stage as *mut _ as *mut u8,
            struct_size,
        )
    };
    file.read_exact(buffer)?;
    Ok(stage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn import_stage_size_matches_cpp_struct() {
        assert_eq!(mem::size_of::<ImportStage>(), 36_732);
    }

    #[test]
    fn load_classic_stage_1() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../base/stages/classic.lvl");
        let stage = load_stage(path.to_str().unwrap(), 1).expect("classic.lvl stage 1 should load");

        let non_empty_tiles: usize = stage
            .array
            .iter()
            .flat_map(|col| col.iter())
            .filter(|&&id| id != 0)
            .count();
        assert!(non_empty_tiles > 0, "stage 1 should contain tiles");

        assert!(stage.start_position_x[0] >= 0);
        assert!(stage.start_position_y[0] >= 0);

        let active_enemies = stage.enemy_in_use.iter().filter(|&&used| used != 0).count();
        assert!(active_enemies > 0, "stage 1 should contain enemies");
    }

    #[test]
    fn load_out_of_bounds_stage_fails() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../base/stages/classic.lvl");
        assert!(load_stage(path.to_str().unwrap(), 999).is_err());
    }

    #[test]
    fn all_33_classic_stages_load() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../base/stages/classic.lvl");
        for stage_num in 1..=33 {
            let stage = load_stage(path.to_str().unwrap(), stage_num)
                .unwrap_or_else(|e| panic!("classic.lvl stage {} failed to load: {}", stage_num, e));
            let has_tiles = stage.array.iter().flat_map(|c| c.iter()).any(|&id| id != 0);
            assert!(has_tiles, "classic.lvl stage {} has no tiles", stage_num);
        }
    }
}
