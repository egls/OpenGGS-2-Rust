// rust_sdl_bridge.h
#ifndef RUST_SDL_BRIDGE_H
#define RUST_SDL_BRIDGE_H

#ifdef __cplusplus
extern "C" {
#endif

// The function names and signatures must exactly match those you 
// exposed in Rust (init_sdl2_bridge, quit_sdl2_bridge, etc.).

// Returns 0 on success, <0 on error
int init_sdl2_bridge(void);

typedef struct {
  int Resolution;
  int Volume_Music;
  int Volume_Sound;
} RustBridgeConfig;

// Configure where Rust should resolve game assets from (UTF-8 path).
int rust_bridge_set_asset_root(const char* asset_root_utf8);

// Mirror C++ runtime state in Rust while gameplay is still C++-owned.
int rust_bridge_sync_runtime_state(unsigned char quit_program,
                                   RustBridgeConfig config);

// No return value
void quit_sdl2_bridge(void);

#ifdef __cplusplus
}
#endif

#endif // RUST_SDL_BRIDGE_H
