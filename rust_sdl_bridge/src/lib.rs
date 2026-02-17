use std::ffi::{c_char, CStr};
use std::path::PathBuf;
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
static mut RUNTIME_STATE: *mut RuntimeBridgeState = ptr::null_mut();

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct RustBridgeConfig {
    pub resolution: i32,
    pub volume_music: i32,
    pub volume_sound: i32,
}

struct RuntimeBridgeState {
    quit_program: bool,
    config: RustBridgeConfig,
    asset_root: PathBuf,
}

impl Default for RuntimeBridgeState {
    fn default() -> Self {
        Self {
            quit_program: false,
            config: RustBridgeConfig::default(),
            asset_root: PathBuf::from("base"),
        }
    }
}

unsafe fn runtime_state_mut() -> &'static mut RuntimeBridgeState {
    unsafe {
        if RUNTIME_STATE.is_null() {
            RUNTIME_STATE = Box::into_raw(Box::new(RuntimeBridgeState::default()));
        }
        &mut *RUNTIME_STATE
    }
}

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
        // Codec init is optional for core startup; open_audio is the hard requirement.
        eprintln!("[rust_sdl_bridge] SDL_mixer codec init warning: {err}");
    }

    if let Err(err) = open_audio(44_100, AUDIO_S16LSB, DEFAULT_CHANNELS, 1024) {
        // Keep startup alive even when audio device init fails; legacy C++ path also retries.
        eprintln!("[rust_sdl_bridge] SDL_mixer open_audio warning: {err}");
    } else {
        allocate_channels(32);
    }

    unsafe {
        SDL_CONTEXT = Box::into_raw(Box::new(sdl_context));
        SDL_IMAGE_CONTEXT = Box::into_raw(Box::new(img_context));
        SDL_MIXER_INITIALIZED = true;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_bridge_set_asset_root(asset_root_utf8: *const c_char) -> i32 {
    if asset_root_utf8.is_null() {
        return -1;
    }

    let path = unsafe {
        match CStr::from_ptr(asset_root_utf8).to_str() {
            Ok(value) => value,
            Err(_) => return -2,
        }
    };

    let state = unsafe { runtime_state_mut() };
    state.asset_root = PathBuf::from(path);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_bridge_sync_runtime_state(
    quit_program: u8,
    config: RustBridgeConfig,
) -> i32 {
    let state = unsafe { runtime_state_mut() };
    state.quit_program = quit_program != 0;
    state.config = config;
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

        if !RUNTIME_STATE.is_null() {
            drop(Box::from_raw(RUNTIME_STATE));
            RUNTIME_STATE = ptr::null_mut();
        }
    }
}
