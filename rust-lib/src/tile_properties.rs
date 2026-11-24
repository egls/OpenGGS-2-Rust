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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn tilesheet_info_size_matches_tsi_file() {
        assert_eq!(mem::size_of::<TileSheetInfo>(), 30_188);
    }

    #[test]
    fn load_tilesheet_properties_from_base() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../base/Tilesheetinfo.tsi");
        let info = load_tile_properties(path.to_str().unwrap())
            .expect("Tilesheetinfo.tsi should load");

        assert_eq!(info.tile_width, 16);
        assert_eq!(info.tile_height, 16);
        assert!(info.solid.iter().any(|&v| v != 0), "expected solid tiles");
        assert!(info.exit.iter().any(|&v| v != 0), "expected exit tiles");
        assert!(info.coin.iter().any(|&v| v != 0), "expected coin tiles");
        assert!(info.lethal.iter().any(|&v| v != 0), "expected lethal tiles");
    }
}
