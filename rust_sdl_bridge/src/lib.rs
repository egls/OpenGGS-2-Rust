// src/lib.rs
use sdl2::Sdl;

// Keep SDL context as a global or in a struct so it doesn't go out of scope.
// For a real program, you might store this in a struct you manage more carefully.
static mut SDL_CONTEXT: Option<Sdl> = None;

// #[no_mangle] ensures that the function names are not changed by Rust’s name mangling.
#[unsafe(no_mangle)]
pub extern "C" fn init_sdl2_bridge() -> i32 {
    println!("init_sdl2_bridge");
    match sdl2::init() {
        Ok(ctx) => {
            unsafe {
                SDL_CONTEXT = Some(ctx);
            }
            0 // Return 0 on success
        },
        Err(_) => -1 // Return error code on failure
    }
}

#[unsafe(no_mangle)] 
pub extern "C" fn quit_sdl2_bridge() { // extern "C" ensures a C-compatible calling convention.
    println!("quit_sdl2_bridge");
    // Not strictly necessary since dropping SDL context is enough,
    // but we can explicitly drop our reference:
    unsafe {
        SDL_CONTEXT = None;
    }
}
