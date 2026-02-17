# OpenGGS-2-Rust — Status (February 17, 2026)

## Snapshot
- The executable still boots the original SDL2 C++ game; Rust is only touched via the FFI stub at `src/main.cpp:18-52`, after which the legacy globals (`QuitProgram`, `Config`) and loops keep running unchanged.
- CMake vendors SDL2/SDL_image/SDL_mixer via `FetchContent` and compiles the 50+ translation units enumerated at `CMakeLists.txt:16-225`, so the project remains primarily CMake-driven C++. The earlier attempt to integrate Rust through Corrosion is still commented out at `CMakeLists.txt:89-101`.
- Assets continue to live under `base/`, and even the Rust bridge accesses them via relative paths such as `base/audio/died.wav` (`rust_sdl_bridge/src/lib.rs:20`).

## Rust work so far
- `rust_sdl_bridge/src/lib.rs:1-55` defines a `staticlib` with `init_sdl2_bridge`/`quit_sdl2_bridge`, but the functions merely call `sdl2::init()`, print diagnostics, and try to synchronously play `base/audio/died.wav` without initializing SDL_mixer. The attribute syntax `#[unsafe(no_mangle)]` is invalid, so the crate does not currently compile without fixes.
- `src/main.cpp:27-35` still contains TODOs about calling Rust from C++ and immediately tears down SDL through the bridge before invoking the long-standing `init()`/`LOOP_Menu()` path, which means no other Rust logic executes.
- The `rust-lib` crate is only a hello-world binary (`rust-lib/src/main.rs:1-6`) and is never linked; `rust_sdl_bridge/Cargo.toml:1-10` is the only Rust manifest that participates in the build.

## Tooling & configuration
- Custom commands at `CMakeLists.txt:194-265` invoke `cargo build --release` to produce either `librust_sdl_bridge.a` or `rust_sdl_bridge.lib`, but the Linux block still has a TODO about archive naming and neither block checks whether `cargo` exists on the host.
- The devcontainer configuration (`.devcontainer/devcontainer.json:1-32`) installs VS Code C++ extensions but does not provision a Rust toolchain, so anyone using it must run `rustup` manually before building the bridge.
- Global headers such as `src/globals.h:2-37` still include Windows-specific `dirent` shims and pull in every subsystem header, highlighting how entangled the runtime state is today.
- The top-level README remains the original OpenGGS blurb with no mention of the Rust port (`README.md:1-9`).

## Key issues / risks
- The SDL context created in Rust is dropped immediately when control returns to C++, so the current init/quit calls are effectively no-ops aside from printing logs (`src/main.cpp:20-35` and `rust_sdl_bridge/src/lib.rs:10-40`).
- Resource paths are hard-coded (e.g., `base/audio/died.wav`), preventing the bridge from working when the working directory changes.
- There are no automated tests or CI hooks—CMake only defines a `format` target (`CMakeLists.txt:170-189`)—so regressions are caught manually.
- Every gameplay, rendering, editor, and physics module listed in `CMakeLists.txt:103-162` still exists only in C++, so the actual game logic has not begun migrating to Rust.
