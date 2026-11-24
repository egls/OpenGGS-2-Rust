use std::path::{Path, PathBuf};

use bevy::prelude::*;

/// Represents a discovered `.lvl` stage pack inside `base/stages/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagePack {
    pub name: String,
    pub path: PathBuf,
}

/// Resolved filesystem root for the `base/` asset directory.
#[derive(Resource, Debug, Clone)]
pub struct AssetPaths {
    pub base_dir: PathBuf,
}

impl AssetPaths {
    pub fn resolve() -> Self {
        Self {
            base_dir: resolve_base_dir(),
        }
    }

    pub fn stages_dir(&self) -> PathBuf {
        self.base_dir.join("stages")
    }

    pub fn tilesheet_info(&self) -> PathBuf {
        self.base_dir.join("Tilesheetinfo.tsi")
    }

    pub fn options_file(&self) -> PathBuf {
        self.base_dir.join("Options.oi")
    }

    pub fn music_file(&self, name: &str) -> PathBuf {
        self.base_dir.join("music").join(name)
    }

    pub fn c64_file(&self, name: &str) -> PathBuf {
        self.base_dir.join("c64").join(name)
    }

    pub fn modules_file(&self) -> PathBuf {
        self.stages_dir().join("modules.lvl")
    }
}

fn is_valid_base_dir(path: &Path) -> bool {
    path.is_dir() && path.join("Tilesheetinfo.tsi").is_file()
}

/// Resolves the `base/` asset directory regardless of working directory or binary location.
pub fn resolve_base_dir() -> PathBuf {
    if let Ok(env_dir) = std::env::var("OPENGGS_BASE_DIR") {
        let p = PathBuf::from(env_dir);
        if is_valid_base_dir(&p) {
            return p;
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("base"));
        if let Some(parent) = cwd.parent() {
            candidates.push(parent.join("base"));
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent();
        for _ in 0..5 {
            if let Some(d) = dir {
                candidates.push(d.join("base"));
                dir = d.parent();
            } else {
                break;
            }
        }
    }

    candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../base"));

    for candidate in candidates {
        if is_valid_base_dir(&candidate) {
            return candidate.canonicalize().unwrap_or(candidate);
        }
    }

    PathBuf::from("../base")
}

/// Formats a `.lvl` file stem into a clean display label (e.g. `my_pack` -> `My Pack`).
pub fn format_pack_name(stem: &str) -> String {
    stem.split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Scans `base/stages/` for all `.lvl` files, keeping built-in packs first in canonical order.
pub fn discover_stage_packs(stages_dir: &Path) -> Vec<StagePack> {
    let mut packs: Vec<StagePack> = Vec::new();
    let Ok(entries) = std::fs::read_dir(stages_dir) else {
        return packs;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let is_lvl = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("lvl"))
            .unwrap_or(false);
        if !is_lvl {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Stage");
        packs.push(StagePack {
            name: format_pack_name(stem),
            path,
        });
    }

    let priority = |p: &StagePack| -> (u8, String) {
        let fname = p
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let rank = match fname.as_str() {
            "classic.lvl" => 0,
            "new.lvl" => 1,
            "modules.lvl" => 2,
            _ => 3,
        };
        (rank, fname)
    };

    packs.sort_by_key(priority);
    packs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_valid_base_directory() {
        let paths = AssetPaths::resolve();
        assert!(paths.tilesheet_info().is_file(), "Tilesheetinfo.tsi not found at {:?}", paths.tilesheet_info());
        assert!(paths.options_file().is_file(), "Options.oi not found");
        assert!(paths.c64_file("Tiles.png").is_file(), "Tiles.png not found");
        assert!(paths.music_file("title.it").is_file(), "title.it not found");
    }

    #[test]
    fn discovers_stage_packs_with_classic_first() {
        let paths = AssetPaths::resolve();
        let packs = discover_stage_packs(&paths.stages_dir());
        assert!(packs.len() >= 3, "expected at least classic, new, and modules packs");
        assert_eq!(packs[0].name, "Classic");
        assert_eq!(packs[1].name, "New");
        assert_eq!(packs[2].name, "Modules");
    }

    #[test]
    fn formats_stage_pack_names_cleanly() {
        assert_eq!(format_pack_name("classic"), "Classic");
        assert_eq!(format_pack_name("custom_world-2"), "Custom World 2");
    }
}
