use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
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

/// Writes a single `ImportStage` record at `stage_index` inside a `.lvl` pack without truncating
/// other stages (matches C++ `STAGE_Save` in `src/CONTENT_Stages_Load_Write.cpp:100-105`).
pub fn save_stage(file_path: &str, stage_index: usize, stage: &ImportStage) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(file_path)?;
    let struct_size = mem::size_of::<ImportStage>();
    file.seek(SeekFrom::Start((stage_index * struct_size) as u64))?;
    let buffer = unsafe {
        std::slice::from_raw_parts(stage as *const ImportStage as *const u8, struct_size)
    };
    file.write_all(buffer)?;
    Ok(())
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
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("base/stages/classic.lvl");
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
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("base/stages/classic.lvl");
        assert!(load_stage(path.to_str().unwrap(), 999).is_err());
    }

    #[test]
    fn all_33_classic_stages_and_substages_load() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("base/stages/classic.lvl");
        for stage_num in 1..=37 {
            let stage = load_stage(path.to_str().unwrap(), stage_num)
                .unwrap_or_else(|e| panic!("classic.lvl stage {} failed to load: {}", stage_num, e));
            let has_tiles = stage.array.iter().flat_map(|c| c.iter()).any(|&id| id != 0);
            assert!(has_tiles, "classic.lvl stage {} has no tiles", stage_num);
        }
    }

    #[test]
    fn all_stage_packs_load_stage_1() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("base/stages");
        for pack in ["classic.lvl", "new.lvl", "modules.lvl"] {
            let path = dir.join(pack);
            let stage = load_stage(path.to_str().unwrap(), 1)
                .unwrap_or_else(|e| panic!("{} stage 1 failed: {}", pack, e));
            let has_tiles = stage.array.iter().flat_map(|c| c.iter()).any(|&id| id != 0);
            assert!(has_tiles, "{} stage 1 has no tiles", pack);
        }
    }

    #[test]
    fn save_stage_round_trips_without_corrupting_adjacent_stages() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("base/stages/classic.lvl");
        let tmp = std::env::temp_dir().join(format!(
            "openggs_stage_save_test_{}.lvl",
            std::process::id()
        ));
        std::fs::copy(&src, &tmp).expect("copy classic.lvl to temp");

        let original_s2 = load_stage(tmp.to_str().unwrap(), 2).unwrap();
        let mut modified_s1 = load_stage(tmp.to_str().unwrap(), 1).unwrap();
        modified_s1.array[10][10] = 501;
        modified_s1.background_colour = 1;
        modified_s1.enemy_in_use[0] = 1;
        modified_s1.enemy_type[0] = 5;
        modified_s1.enemy_pos_x[0] = 320;
        modified_s1.enemy_pos_y[0] = 240;

        save_stage(tmp.to_str().unwrap(), 1, &modified_s1).expect("save_stage should succeed");

        let reloaded_s1 = load_stage(tmp.to_str().unwrap(), 1).unwrap();
        let reloaded_s2 = load_stage(tmp.to_str().unwrap(), 2).unwrap();
        let _ = std::fs::remove_file(&tmp);

        assert_eq!(reloaded_s1.array[10][10], 501);
        assert_eq!(reloaded_s1.background_colour, 1);
        assert_eq!(reloaded_s1.enemy_type[0], 5);
        assert_eq!(reloaded_s1.enemy_pos_x[0], 320);
        assert_eq!(reloaded_s2.array, original_s2.array);
    }
}
