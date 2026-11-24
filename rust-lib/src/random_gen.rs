use std::io;
use std::path::Path;

use bevy::prelude::*;

use crate::level::{load_stage, ImportStage};
use crate::tile_properties::TileSheetInfo;
use crate::tilemap::{STAGE_COLS, STAGE_ROWS, TILES_PER_ROW};

pub const MODULE_WIDTH: usize = 32;
pub const MODULE_HEIGHT: usize = 30;
const MAX_NUM_ENEMIES: usize = 50;

#[derive(Debug, Clone, Copy)]
pub struct ModuleEnemy {
    pub enemy_type: i32,
    pub direction:  i32,
    pub rel_tile_x: usize,
    pub tile_y:     usize,
}

#[derive(Debug, Clone)]
pub struct StageModule {
    pub tiles:   [[i32; MODULE_HEIGHT]; MODULE_WIDTH],
    pub enemies: Vec<ModuleEnemy>,
}

/// Bank of 32×30 terrain & exit modules extracted from `base/stages/modules.lvl`
/// (matches C++ `STAGES_Import_RANDOM_Modules` in `CONTENT_Stages_Load_Write.cpp:486-831`).
#[derive(Resource, Debug, Clone, Default)]
pub struct RandomModuleBank {
    pub outdoors:      Vec<StageModule>,
    pub outdoors_exit: Vec<StageModule>,
    pub indoors:       Vec<StageModule>,
    pub indoors_exit:  Vec<StageModule>,
}

impl RandomModuleBank {
    pub fn load_from_file(path: &Path) -> io::Result<Self> {
        let path_str = path.to_string_lossy();
        let mut bank = Self::default();

        // Outdoor normal modules: stages 1..=16 (8 modules per stage)
        for stage_idx in 1..=16 {
            if let Ok(stage) = load_stage(&path_str, stage_idx) {
                extract_stage_modules(&stage, &mut bank.outdoors);
            }
        }

        // Indoor normal modules: stages 17..=32 (8 modules per stage)
        for stage_idx in 17..=32 {
            if let Ok(stage) = load_stage(&path_str, stage_idx) {
                extract_stage_modules(&stage, &mut bank.indoors);
            }
        }

        // Outdoor exit modules: stage 34
        if let Ok(stage) = load_stage(&path_str, 34) {
            extract_stage_modules(&stage, &mut bank.outdoors_exit);
        }

        // Indoor exit modules: stage 35
        if let Ok(stage) = load_stage(&path_str, 35) {
            extract_stage_modules(&stage, &mut bank.indoors_exit);
        }

        Ok(bank)
    }

    pub fn is_ready(&self) -> bool {
        !self.outdoors.is_empty()
            && !self.outdoors_exit.is_empty()
            && !self.indoors.is_empty()
            && !self.indoors_exit.is_empty()
    }
}

fn extract_stage_modules(stage: &ImportStage, dest: &mut Vec<StageModule>) {
    for slot in 0..8usize {
        let x_start = slot * MODULE_WIDTH;
        let mut tiles = [[0i32; MODULE_HEIGHT]; MODULE_WIDTH];
        let mut has_any_tile = false;

        for lx in 0..MODULE_WIDTH {
            for y in 0..MODULE_HEIGHT {
                let id = stage.array[x_start + lx][y];
                tiles[lx][y] = id;
                if id != 0 {
                    has_any_tile = true;
                }
            }
        }

        if !has_any_tile {
            continue;
        }

        let mut enemies = Vec::new();
        for i in 0..MAX_NUM_ENEMIES {
            if stage.enemy_in_use[i] == 0 {
                continue;
            }
            let tile_x = (stage.enemy_pos_x[i] / 16).max(0) as usize;
            let tile_y = (stage.enemy_pos_y[i] / 16).clamp(0, (MODULE_HEIGHT - 1) as i32) as usize;
            if tile_x >= x_start && tile_x < x_start + MODULE_WIDTH {
                enemies.push(ModuleEnemy {
                    enemy_type: stage.enemy_type[i],
                    direction:  stage.enemy_direction[i],
                    rel_tile_x: tile_x - x_start,
                    tile_y,
                });
            }
        }

        dest.push(StageModule { tiles, enemies });
    }
}

fn next_rand(state: &mut u64) -> usize {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*state >> 32) as usize
}

/// Returns `(width, height)` in tiles for multi-tile decorative elements (`GRAPHICS_Tiles_Elements.cpp`).
pub fn element_dimensions(base_tile: i32) -> (usize, usize) {
    match base_tile {
        566 => (2, 1),
        3 | 87 | 670 | 672 | 674 | 676 | 678 => (2, 2),
        780 | 788 | 791 | 794 => (2, 3),
        250 | 630 | 633 | 636 => (3, 1),
        80 | 480 | 501 | 560 | 581 | 587 | 640 => (3, 2),
        32 | 50 | 186 | 511 | 797 => (3, 3),
        726 => (3, 4),
        29 | 269 => (3, 5),
        10 => (4, 1),
        6 | 83 => (4, 2),
        60 | 177 | 775 | 783 => (4, 3),
        16 | 729 => (4, 4),
        104 => (5, 2),
        181 => (5, 3),
        170 => (6, 2),
        514 => (6, 3),
        720 => (6, 4),
        272 => (8, 6),
        160 => (10, 3),
        _ => (1, 1),
    }
}

/// Stamps a multi-tile element whose tiles are laid out consecutively in `Tiles.png`.
pub fn stamp_element(stage: &mut ImportStage, base_tile: i32, start_x: usize, start_y: usize) {
    let (w, h) = element_dimensions(base_tile);
    for dy in 0..h {
        for dx in 0..w {
            let tx = start_x + dx;
            let ty = start_y + dy;
            if tx < STAGE_COLS && ty < STAGE_ROWS {
                stage.array[tx][ty] = base_tile + dx as i32 + (dy as i32 * TILES_PER_ROW);
            }
        }
    }
}

/// Generates a full playable 256×30 stage from `RandomModuleBank`
/// (matches C++ `LOOP_Editor_Stages_Create` in `src/LOOP_Editor_Stages_Creator.cpp`).
pub fn generate_random_stage(
    bank:              &RandomModuleBank,
    stage_number:      usize,
    background_colour: i32,
    tile_info:         &TileSheetInfo,
    seed:              u64,
) -> Option<ImportStage> {
    if !bank.is_ready() {
        return None;
    }

    let mut rng = seed
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(stage_number as u64 + 1);

    let mut stage = ImportStage::new();
    stage.number = stage_number as i32;
    stage.background_colour = if background_colour == 0 { 0 } else { 1 };
    stage.tile_type = if stage.background_colour == 0 {
        0
    } else {
        1 + (next_rand(&mut rng) % 3) as i32
    };

    let label = format!("Random Stage {:02}", stage_number);
    let bytes = label.as_bytes();
    let len = bytes.len().min(stage.name.len() - 1);
    stage.name[..len].copy_from_slice(&bytes[..len]);

    let (normal_pool, exit_pool) = if stage.background_colour == 0 {
        (&bank.outdoors, &bank.outdoors_exit)
    } else {
        (&bank.indoors, &bank.indoors_exit)
    };

    let mut enemy_slot = 0usize;

    // Stamp 7 normal modules (slots 0..7) + 1 exit module (slot 7)
    for slot in 0..8usize {
        let module = if slot < 7 {
            &normal_pool[next_rand(&mut rng) % normal_pool.len()]
        } else {
            &exit_pool[next_rand(&mut rng) % exit_pool.len()]
        };

        let x_offset = slot * MODULE_WIDTH;
        for lx in 0..MODULE_WIDTH {
            for y in 0..MODULE_HEIGHT {
                let id = module.tiles[lx][y];
                if id != 0 {
                    stage.array[x_offset + lx][y] = id;
                }
            }
        }

        for e in &module.enemies {
            if enemy_slot >= MAX_NUM_ENEMIES {
                break;
            }
            stage.enemy_in_use[enemy_slot] = 1;
            stage.enemy_type[enemy_slot] = e.enemy_type;
            stage.enemy_direction[enemy_slot] = e.direction;
            stage.enemy_pos_x[enemy_slot] = ((x_offset + e.rel_tile_x) * 16) as i32;
            stage.enemy_pos_y[enemy_slot] = (e.tile_y * 16) as i32;
            enemy_slot += 1;
        }
    }

    // Add decorative outdoor plants over solid ground (matches LOOP_Editor_Stages_Creator.cpp:78-114)
    if stage.background_colour == 0 {
        const PLANTS: [i32; 6] = [160, 170, 177, 181, 186, 60];
        for _ in 0..16 {
            let plant = PLANTS[next_rand(&mut rng) % PLANTS.len()];
            let (pw, ph) = element_dimensions(plant);
            let pos_x = 12 + (next_rand(&mut rng) % (224 - pw - 12));
            let base_y = 27usize;

            let solid_ground = (0..pw).all(|dx| is_solid_tile(&stage, tile_info, pos_x + dx, base_y));
            let clear_above = (0..pw).all(|dx| {
                (base_y.saturating_sub(ph)..base_y).all(|y| stage.array[pos_x + dx][y] == 0)
            });
            if solid_ground && clear_above {
                stamp_element(&mut stage, plant, pos_x, base_y - ph);
            }
        }
    }

    // Place safe start/checkpoint positions (matches LOOP_Editor_Stages_Creator.cpp:116-144)
    let start_cols = [5usize, 100, 200, 4];
    for (cp, &col) in start_cols.iter().enumerate() {
        let mut y = 26usize;
        while y > 4
            && (is_solid_tile(&stage, tile_info, col, y)
                || is_solid_tile(&stage, tile_info, col + 1, y)
                || is_solid_tile(&stage, tile_info, col, y - 1)
                || is_solid_tile(&stage, tile_info, col + 1, y - 1)
                || is_solid_tile(&stage, tile_info, col, y - 2)
                || is_solid_tile(&stage, tile_info, col + 1, y - 2))
        {
            y -= 1;
        }

        let hazardous = y >= 26
            || is_lethal_tile(&stage, tile_info, col, y)
            || is_lethal_tile(&stage, tile_info, col.saturating_sub(1), y)
            || is_lethal_tile(&stage, tile_info, col + 1, y)
            || !is_solid_tile(&stage, tile_info, col, y + 1);

        if hazardous {
            y = 18;
            // Place a safe landing shelf directly beneath spawn
            for sx in [col.saturating_sub(2), col, col + 2] {
                stamp_element(&mut stage, 566, sx, y + 3);
            }
        }

        // Convert top-left spawn tile to pixel coordinates compatible with `player_start_position`
        stage.start_position_x[cp] = (col * 16) as i32;
        stage.start_position_y[cp] = (y.saturating_sub(2) * 16) as i32;
    }

    Some(stage)
}

fn is_solid_tile(stage: &ImportStage, tile_info: &TileSheetInfo, tx: usize, ty: usize) -> bool {
    if tx >= STAGE_COLS || ty >= STAGE_ROWS {
        return false;
    }
    let id = stage.array[tx][ty];
    id > 0 && (id as usize) < 2320 && tile_info.solid[id as usize] != 0
}

fn is_lethal_tile(stage: &ImportStage, tile_info: &TileSheetInfo, tx: usize, ty: usize) -> bool {
    if tx >= STAGE_COLS || ty >= STAGE_ROWS {
        return false;
    }
    let id = stage.array[tx][ty];
    id > 0 && (id as usize) < 2320 && tile_info.lethal[id as usize] != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::AssetPaths;
    use crate::tile_properties::load_tile_properties;

    #[test]
    fn loads_all_four_module_pools_from_modules_lvl() {
        let paths = AssetPaths::resolve();
        let bank = RandomModuleBank::load_from_file(&paths.modules_file())
            .expect("modules.lvl should load");

        assert!(bank.is_ready(), "all four module pools should be populated");
        assert!(!bank.outdoors.is_empty());
        assert!(!bank.outdoors_exit.is_empty());
        assert!(!bank.indoors.is_empty());
        assert!(!bank.indoors_exit.is_empty());
    }

    #[test]
    fn generates_valid_outdoor_and_indoor_stages_with_exits_and_safe_spawns() {
        let paths = AssetPaths::resolve();
        let bank = RandomModuleBank::load_from_file(&paths.modules_file()).unwrap();
        let tile_info = load_tile_properties(paths.tilesheet_info().to_str().unwrap()).unwrap();

        for (stage_num, bg) in [(1usize, 0i32), (2usize, 1i32)] {
            let stage = generate_random_stage(&bank, stage_num, bg, &tile_info, 12345)
                .expect("should generate random stage");

            assert_eq!(stage.background_colour, bg);

            // Must have an exit tile in the final exit module (cols 224..256)
            let has_exit = (224..STAGE_COLS).any(|x| {
                (0..STAGE_ROWS).any(|y| {
                    let id = stage.array[x][y];
                    id > 0 && (id as usize) < 2320 && tile_info.exit[id as usize] != 0
                })
            });
            assert!(has_exit, "generated stage (bg={}) missing exit tile in final module", bg);

            // Primary spawn should be valid and not buried inside solid tiles
            let sx = (stage.start_position_x[0] / 16) as usize;
            let sy = (stage.start_position_y[0] / 16) as usize;
            assert!(!is_solid_tile(&stage, &tile_info, sx, sy));

            // All active enemies must have valid types and in-bounds coordinates
            for i in 0..MAX_NUM_ENEMIES {
                if stage.enemy_in_use[i] != 0 {
                    assert!((1..=15).contains(&stage.enemy_type[i]));
                    assert!((0..(STAGE_COLS as i32 * 16)).contains(&stage.enemy_pos_x[i]));
                    assert!((0..(STAGE_ROWS as i32 * 16)).contains(&stage.enemy_pos_y[i]));
                }
            }
        }
    }

    #[test]
    fn generation_is_deterministic_per_seed_and_varies_across_seeds() {
        let paths = AssetPaths::resolve();
        let bank = RandomModuleBank::load_from_file(&paths.modules_file()).unwrap();
        let tile_info = load_tile_properties(paths.tilesheet_info().to_str().unwrap()).unwrap();

        let a = generate_random_stage(&bank, 1, 0, &tile_info, 777).unwrap();
        let b = generate_random_stage(&bank, 1, 0, &tile_info, 777).unwrap();
        let c = generate_random_stage(&bank, 1, 0, &tile_info, 999).unwrap();

        assert_eq!(a.array, b.array);
        assert_ne!(a.array, c.array);
    }
}
