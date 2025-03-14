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

// No return value
void quit_sdl2_bridge(void);

#ifdef __cplusplus
}
#endif

#endif // RUST_SDL_BRIDGE_H
