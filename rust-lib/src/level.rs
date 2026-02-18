use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::mem;

const MAX_NUM_ENEMIES: usize = 50;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImportStage {
    pub name: [u8; 100],        // name of the stage (default = "noname")
    pub number: i32,            // every stage has a number
    pub array: [[i32; 30]; 256],    // tile array [x][y]? C++ is [256][30]. 
                                    // In C: array[0] is array of 30 ints.
                                    // So in Rust: [[i32; 30]; 256].
    pub tile_type: i32,          // above ground, underground, different colours...
    pub background_colour: i32,  // background colour (blue or black)
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
    
    // Seek to the correct offset
    file.seek(SeekFrom::Start((stage_index * struct_size) as u64))?;
    
    // Read the data
    let mut stage = ImportStage::new();
    let buffer = unsafe {
        std::slice::from_raw_parts_mut(
            &mut stage as *mut _ as *mut u8,
            struct_size
        )
    };
    
    file.read_exact(buffer)?;
    
    Ok(stage)
}
