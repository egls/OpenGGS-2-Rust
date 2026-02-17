# Runtime Ownership Model

## Decision
The project uses a C++-driven game loop with a Rust-owned SDL runtime layer.

## Current boundary
- C++ owns:
  - game loop (`LOOP_*`)
  - gameplay systems (player, enemies, collisions, stage/editor logic)
  - most game state mutations
- Rust owns:
  - SDL lifecycle (`init_sdl2_bridge`, `quit_sdl2_bridge`)
  - runtime mirror for key bridge state (`QuitProgram`, resolution, volume values)
  - asset root configuration used by Rust-side subsystems

## Direction for the port
1. Keep C++ calling into Rust for runtime/system services during transition.
2. Port one subsystem at a time to Rust (SDL wrappers, then rendering/physics/content).
3. After each subsystem port, remove the matching C++ source files from the CMake `SOURCE_FILES` list.
4. When the game loop and gameplay state are fully in Rust, flip control flow so Rust is the primary entrypoint.
