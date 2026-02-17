use std::ptr;

use sdl2::image::{self, InitFlag as ImageInitFlag, Sdl2ImageContext};
use sdl2::mixer::{
    allocate_channels, close_audio, init as mixer_init, open_audio, InitFlag as MixerInitFlag,
    AUDIO_S16LSB, DEFAULT_CHANNELS,
};
use sdl2::Sdl;

static mut SDL_CONTEXT: *mut Sdl = ptr::null_mut();
static mut SDL_IMAGE_CONTEXT: *mut Sdl2ImageContext = ptr::null_mut();
static mut SDL_MIXER_INITIALIZED: bool = false;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_sdl2_bridge() -> i32 {
    unsafe {
        if !SDL_CONTEXT.is_null() {
            return 0;
        }
    }

    let sdl_context = match sdl2::init() {
        Ok(ctx) => ctx,
        Err(err) => {
            eprintln!("[rust_sdl_bridge] Failed to init SDL: {err}");
            return -1;
        }
    };

    let img_context = match image::init(ImageInitFlag::PNG | ImageInitFlag::JPG) {
        Ok(ctx) => ctx,
        Err(err) => {
            eprintln!("[rust_sdl_bridge] Failed to init SDL_image: {err}");
            return -2;
        }
    };

    if let Err(err) = mixer_init(MixerInitFlag::MP3 | MixerInitFlag::OGG | MixerInitFlag::MOD) {
        eprintln!("[rust_sdl_bridge] Failed to init SDL_mixer: {err}");
        return -3;
    }

    if let Err(err) = open_audio(44_100, AUDIO_S16LSB, DEFAULT_CHANNELS, 1024) {
        eprintln!("[rust_sdl_bridge] Failed to open SDL_mixer audio: {err}");
        return -4;
    }
    allocate_channels(32);

    unsafe {
        SDL_CONTEXT = Box::into_raw(Box::new(sdl_context));
        SDL_IMAGE_CONTEXT = Box::into_raw(Box::new(img_context));
        SDL_MIXER_INITIALIZED = true;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn quit_sdl2_bridge() {
    unsafe {
        if SDL_MIXER_INITIALIZED {
            close_audio();
            SDL_MIXER_INITIALIZED = false;
        }

        if !SDL_IMAGE_CONTEXT.is_null() {
            drop(Box::from_raw(SDL_IMAGE_CONTEXT));
            SDL_IMAGE_CONTEXT = ptr::null_mut();
        }

        if !SDL_CONTEXT.is_null() {
            drop(Box::from_raw(SDL_CONTEXT));
            SDL_CONTEXT = ptr::null_mut();
        }
    }
}
