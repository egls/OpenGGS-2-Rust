use std::fs::File;
use std::io::{self, Read, Write};
use std::mem;

pub const NUMBER_OF_TILES: usize = 2320;

/// All 13 editable per-tile boolean flags in `Tilesheetinfo.tsi`
/// (matches C++ `LOOP_Editor_Tiles.cpp:102-156`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileFlag {
    Solid,
    Exit,
    Lethal,
    Fire,
    Coin,
    CoinBlock,
    Breakable,
    PowerUpBlock,
    CoinBlockHelmet,
    ColourChanging,
    SubStageEntrance,
    DropStone,
    WarpStone,
}

impl TileFlag {
    pub const ALL: [TileFlag; 13] = [
        Self::Solid,
        Self::Exit,
        Self::Lethal,
        Self::Fire,
        Self::Coin,
        Self::CoinBlock,
        Self::Breakable,
        Self::PowerUpBlock,
        Self::CoinBlockHelmet,
        Self::ColourChanging,
        Self::SubStageEntrance,
        Self::DropStone,
        Self::WarpStone,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Solid            => "Solid",
            Self::Exit             => "Exit",
            Self::Lethal           => "Lethal",
            Self::Fire             => "Fire",
            Self::Coin             => "Coin",
            Self::CoinBlock        => "CoinBlock",
            Self::Breakable        => "Breakable",
            Self::PowerUpBlock     => "PowerUpBlock",
            Self::CoinBlockHelmet  => "CoinBlock (Helmet)",
            Self::ColourChanging   => "ColourChanging",
            Self::SubStageEntrance => "SubStageEntrance",
            Self::DropStone        => "DropStone",
            Self::WarpStone        => "WarpStone",
        }
    }

    pub fn get(self, info: &TileSheetInfo, tile_id: usize) -> bool {
        if tile_id >= NUMBER_OF_TILES {
            return false;
        }
        let val = match self {
            Self::Solid            => info.solid[tile_id],
            Self::Exit             => info.exit[tile_id],
            Self::Lethal           => info.lethal[tile_id],
            Self::Fire             => info.fire[tile_id],
            Self::Coin             => info.coin[tile_id],
            Self::CoinBlock        => info.coin_block[tile_id],
            Self::Breakable        => info.breakable[tile_id],
            Self::PowerUpBlock     => info.power_up_block[tile_id],
            Self::CoinBlockHelmet  => info.coin_block_helmet[tile_id],
            Self::ColourChanging   => info.colour_changing[tile_id],
            Self::SubStageEntrance => info.sub_stage_entrance[tile_id],
            Self::DropStone        => info.drop_stone[tile_id],
            Self::WarpStone        => info.warp_stone[tile_id],
        };
        val != 0
    }

    pub fn toggle(self, info: &mut TileSheetInfo, tile_id: usize) -> bool {
        if tile_id >= NUMBER_OF_TILES {
            return false;
        }
        let slot = match self {
            Self::Solid            => &mut info.solid[tile_id],
            Self::Exit             => &mut info.exit[tile_id],
            Self::Lethal           => &mut info.lethal[tile_id],
            Self::Fire             => &mut info.fire[tile_id],
            Self::Coin             => &mut info.coin[tile_id],
            Self::CoinBlock        => &mut info.coin_block[tile_id],
            Self::Breakable        => &mut info.breakable[tile_id],
            Self::PowerUpBlock     => &mut info.power_up_block[tile_id],
            Self::CoinBlockHelmet  => &mut info.coin_block_helmet[tile_id],
            Self::ColourChanging   => &mut info.colour_changing[tile_id],
            Self::SubStageEntrance => &mut info.sub_stage_entrance[tile_id],
            Self::DropStone        => &mut info.drop_stone[tile_id],
            Self::WarpStone        => &mut info.warp_stone[tile_id],
        };
        *slot = if *slot == 0 { 1 } else { 0 };
        *slot != 0
    }
}

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

/// Writes `TileSheetInfo` back to `Tilesheetinfo.tsi` in exact C++ binary layout (30,188 bytes).
pub fn save_tile_properties(file_path: &str, info: &TileSheetInfo) -> io::Result<()> {
    let mut file = File::create(file_path)?;
    let size = mem::size_of::<TileSheetInfo>();
    let bytes = unsafe { std::slice::from_raw_parts(info as *const TileSheetInfo as *const u8, size) };
    file.write_all(bytes)?;
    Ok(())
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
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("base/Tilesheetinfo.tsi");
        let info = load_tile_properties(path.to_str().unwrap())
            .expect("Tilesheetinfo.tsi should load");

        assert_eq!(info.tile_width, 16);
        assert_eq!(info.tile_height, 16);
        assert!(info.solid.iter().any(|&v| v != 0), "expected solid tiles");
        assert!(info.exit.iter().any(|&v| v != 0), "expected exit tiles");
        assert!(info.coin.iter().any(|&v| v != 0), "expected coin tiles");
        assert!(info.lethal.iter().any(|&v| v != 0), "expected lethal tiles");
    }

    #[test]
    fn save_tile_properties_round_trips_and_toggles_all_flags() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("base/Tilesheetinfo.tsi");
        let tmp = std::env::temp_dir().join(format!(
            "openggs_tsi_test_{}.tsi",
            std::process::id()
        ));

        let mut info = load_tile_properties(src.to_str().unwrap()).unwrap();
        let test_tile = 123usize;
        let before: Vec<bool> = TileFlag::ALL.iter().map(|f| f.get(&info, test_tile)).collect();
        for flag in TileFlag::ALL {
            flag.toggle(&mut info, test_tile);
        }
        save_tile_properties(tmp.to_str().unwrap(), &info).expect("save_tile_properties failed");

        let meta = std::fs::metadata(&tmp).unwrap();
        assert_eq!(meta.len(), 30_188);

        let reloaded = load_tile_properties(tmp.to_str().unwrap()).unwrap();
        let _ = std::fs::remove_file(&tmp);

        for (flag, prev) in TileFlag::ALL.iter().zip(before.iter()) {
            assert_eq!(flag.get(&reloaded, test_tile), !prev, "flag {:?} did not persist", flag);
        }
    }
}
