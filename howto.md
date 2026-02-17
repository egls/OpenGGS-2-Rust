# How to Build OpenGGS-2-Rust (February 17, 2026)

## 1. Prerequisites
- CMake ≥ 3.15 and a C++17-capable compiler (MSVC, Clang, or GCC), matching the settings at `CMakeLists.txt:1-7`.
- Rust toolchain (stable 1.77+ recommended) with `cargo` to build `rust_sdl_bridge` (`rust_sdl_bridge/Cargo.toml:1-10`). The devcontainer does not install Rust yet (`.devcontainer/devcontainer.json:1-32`), so run `rustup-init` inside it.
- Ninja or Make (choose via `-G`), Python, and Git for CMake’s FetchContent (SDL dependencies are cloned at `CMakeLists.txt:16-70`).
- SDL build prerequisites (CMake will fetch the sources, but system packages such as `libasound2-dev` on Linux may still be needed).

## 2. Configure
```sh
cmake -S . -B build -G "Ninja" -DCMAKE_BUILD_TYPE=Release
```
- The first configure downloads SDL2/SDL_image/SDL_mixer; keep an internet connection or pre-populate the `_deps/` folder referenced by CMake.
- Optional: enable clang-tidy with `-DENABLE_CLANG_TIDY=ON` to use the hook defined at `CMakeLists.txt:74-85`.

## 3. Build
```sh
cmake --build build --target openggs --config Release
```
- During the build, CMake’s custom commands at `CMakeLists.txt:194-265` automatically run `cargo build --release` inside `rust_sdl_bridge/` and link the resulting static library. Make sure `cargo` is on `PATH`; otherwise the build will fail before linking.
- On multi-config generators (Visual Studio), pass `--config Debug`/`Release` to both configure and build commands.

## 4. Run
- The binary will be emitted under `build/` (exact path depends on generator). Keep the `base/` directory next to the executable because both the C++ and Rust layers load assets via relative paths such as `base/audio/died.wav` (`rust_sdl_bridge/src/lib.rs:20`).
- SDL requires the standard DLLs (if you are on Windows) or the shared libraries built under `build/_deps/` (Linux). Ensure they are discoverable via `PATH`/`LD_LIBRARY_PATH`.

## 5. Troubleshooting
- If CMake cannot find `cargo`, install Rust and reconfigure so the custom bridge target can succeed.
- To rebuild after changing Rust code only, you can run `cargo build --release` inside `rust_sdl_bridge/` directly; the next `cmake --build` will re-link if timestamps have changed.
- Use `cmake --build build --target format` to run clang-format on the C++ sources as defined at `CMakeLists.txt:170-189`.
- For headless development, open the repo inside the provided devcontainer but remember to forward ports 5901/6080 if you plan to use the optional desktop feature (`.devcontainer/devcontainer.json:21-30`).
