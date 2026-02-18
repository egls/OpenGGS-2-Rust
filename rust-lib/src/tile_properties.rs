use std::fs::File;
use std::io::{self, Read};
use std::mem;

const NUMBER_OF_TILES: usize = 2320;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TileSheetInfo {
    pub tile_width: i32,
    pub tile_height: i32,
    pub width: i32,
    pub height: i32,
    pub width_pixels: i32,
    pub height_pixels: i32,
    pub solid: [u8; NUMBER_OF_TILES],
    pub exit: [u8; NUMBER_OF_TILES],
    pub lethal: [u8; NUMBER_OF_TILES],
    pub fire: [u8; NUMBER_OF_TILES],
    pub coin: [u8; NUMBER_OF_TILES],
    pub coin_block: [u8; NUMBER_OF_TILES],
    pub next_frame: u8,
    pub breakable: [u8; NUMBER_OF_TILES],
    pub power_up_block: [u8; NUMBER_OF_TILES],
    pub coin_block_helmet: [u8; NUMBER_OF_TILES],
    pub colour_changing: [u8; NUMBER_OF_TILES],
    pub sub_stage_entrance: [u8; NUMBER_OF_TILES],
    pub drop_stone: [u8; NUMBER_OF_TILES],
    pub warp_stone: [u8; NUMBER_OF_TILES],
}

impl TileSheetInfo {
    pub fn new() -> Self {
        unsafe { mem::zeroed() }
    }
}

pub fn load_tile_properties(file_path: &str) -> io::Result<TileSheetInfo> {
    let mut file = File::open(file_path)?;
    let mut info = TileSheetInfo::new();
    let buffer_size = mem::size_of::<TileSheetInfo>();
    let mut buffer = vec![0u8; buffer_size];
    file.read_exact(&mut buffer)?;
    unsafe {
        std::ptr::copy_nonoverlapping(
            buffer.as_ptr(),
            &mut info as *mut TileSheetInfo as *mut u8,
            buffer_size,
        );
    }
    Ok(info)
}
