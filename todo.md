# OpenGGS-2-Rust — Porting TODO

## 0. Repo hygiene and build ownership
- [x] Decide which runtime (C++ vs. Rust) will own the SDL bootstrap; Rust now initializes/shuts down SDL via `rust_sdl_bridge/src/lib.rs:1-77` and C++ delegates through `src/main.cpp:20-44`.
- [x] Replace the commented `corrosion_import_crate` block with a working integration (`CMakeLists.txt:90-150` now fetches Corrosion and links `rust_sdl_bridge` as a proper target).
- [x] Ensure the devcontainer provisions Rust (`.devcontainer/devcontainer.json:1-33` adds the official Rust devcontainer feature so `cargo` is ready without manual steps).
- [x] Audit the FetchContent flow for SDL2/SDL_image/SDL_mixer (kept the existing pinned-from-source approach for reproducibility; document this decision here).
- [x] Prune legacy binaries (root-level `openggs.exe` and SDL2 DLL/lib snapshots removed so builds rely on the configured toolchains).

## 1. Rust runtime and FFI bridge
- [x] Fix symbol exports and safety in the Rust bridge and keep SDL contexts alive for the full runtime (`rust_sdl_bridge/src/lib.rs`).
- [x] Mirror key C++ runtime state in Rust via FFI (`RustBridgeConfig` + `rust_bridge_sync_runtime_state` in `src/rust_sdl_bridge.h` and `rust_sdl_bridge/src/lib.rs`).
- [x] Parameterize asset discovery by passing the asset root from C++ (`rust_bridge_set_asset_root` in `src/main.cpp`).
- [x] Lock the ownership direction as C++ loop -> Rust runtime services and document it in `docs/runtime_ownership.md`.
- [x] Harden startup/runtime deployment so the game can launch from `build/` (post-build copy of SDL DLLs + `base/` and non-fatal Rust mixer init path).

## 2. Module-by-module porting
- [ ] Reimplement the SDL wrapper layer now located in `src/SYSTEM_SDL_init.cpp`, `src/SYSTEM_SDL_input.cpp`, `src/SYSTEM_SDL_Textures.cpp`, `src/SYSTEM_Update_Screen.cpp`, and `src/SYSTEM_Vector_Operations.cpp` using safe abstractions over the `sdl2` crate.
- [ ] Port the rendering stack from `src/GRAPHICS_*.cpp` (enumerated in `CMakeLists.txt:108-139`) by modeling sprites, tiles, and animations as Rust enums/structs; verify feature parity (scrolling, animation timers, bullet rendering).
- [ ] Translate gameplay/physics from `src/PLAYER*.cpp`, `src/ENEMIES*.cpp`, and related headers into Rust modules with deterministic state machines so AI and collision behave identically.
- [ ] Move content serialization/editor logic from `src/CONTENT_*.h` and the `src/LOOP_Editor_*` family into Rust, ideally backed by serde-friendly data formats for level packs.
- [ ] Replace filesystem calls guarded by `dirent` in `src/globals.h:6-16` with Rust’s `std::fs`/`walkdir` to simplify cross-platform support.

## 3. Tooling, testing, and CI
- [ ] Introduce unit/integration tests once the SDL wrapper lands; CMake currently lacks `enable_testing()` or `add_test()`, so create a Cargo test crate or add gtests to catch regressions (`CMakeLists.txt:170-189` only defines `format` today).
- [ ] Add continuous integration for Windows and Linux to run `cmake -S . -B build` and `cargo test` so both toolchains stay green when porting modules.
- [ ] Capture performance baselines (player movement, collision loops) before rewriting them so you can compare Rust vs. C++ behaviour during migration.

## 4. Migration & documentation
- [ ] As Rust equivalents become available, remove the corresponding C++ files from the `SOURCE_FILES` block (`CMakeLists.txt:103-162`) so they stop compiling twice.
- [ ] Update the top-level README once the Rust runtime can launch the game (current `README.md:1-9` still describes the original C++ fork only) and reference the new docs produced here.
- [ ] Retire the `rust_sdl_bridge` shim once the whole application runs as a native Rust binary, or clearly document any remaining C++ subsystems that will stay behind.
