# OpenGGS-2-Rust - Status (February 17, 2026)

## Snapshot
- Runtime ownership for SDL now sits in Rust (`rust_sdl_bridge/src/lib.rs`), while gameplay loops and content logic remain in C++ (`src/main.cpp`, `src/LOOP_*.cpp`).
- CMake now imports the Rust bridge via Corrosion instead of raw custom `cargo build` commands (`CMakeLists.txt`).
- The Windows build uses the GNU Rust target triple to match the existing LLVM/MinGW C++ toolchain, and `openggs` currently builds successfully.

## Rust bridge state
- `init_sdl2_bridge` initializes SDL, SDL_image, and SDL_mixer and keeps contexts alive until `quit_sdl2_bridge`.
- Rust now mirrors C++ runtime state through FFI:
  - `rust_bridge_sync_runtime_state` receives `QuitProgram` and `Config_Type`-equivalent fields.
  - `rust_bridge_set_asset_root` receives a UTF-8 asset path from C++.
- C++ calls these sync functions from `main` after loading options and before shutdown.

## Build/tooling state
- Devcontainer now provisions Rust via devcontainer features (`.devcontainer/devcontainer.json`).
- CMake pins SDL2/SDL2_image/SDL2_mixer via `FetchContent` and links Rust bridge + SDL targets into `openggs`.
- Rust target setup on Windows expects `x86_64-pc-windows-gnu`.

## Ownership model (current)
- Rust owns platform/runtime concerns for SDL lifecycle and bridge state.
- C++ still owns game loop, gameplay logic, editors, and most content/data flow.
- Porting strategy is incremental: move subsystems from C++ into Rust, then remove corresponding C++ source units from CMake.

## Remaining major risks
- The Rust state mirror is currently synchronized at key points, not continuously across every C++ mutation path.
- Most gameplay, physics, rendering, and editor code is still C++ and not yet ported.
- Build warnings remain in legacy C++ sources (format-string and source-encoding warnings).
