mod assets;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::time::Duration;
use crate::assets::AssetManager;

pub fn main() -> Result<(), String> {
    env_logger::init();
    
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let _image_context = sdl2::image::init(sdl2::image::InitFlag::PNG | sdl2::image::InitFlag::JPG)?;
    let _audio_subsystem = sdl_context.audio()?;
    let _mixer_context = sdl2::mixer::init(sdl2::mixer::InitFlag::OGG | sdl2::mixer::InitFlag::MP3)?;

    // Open Mixer
    sdl2::mixer::open_audio(44100, sdl2::mixer::AUDIO_S16LSB, 2, 1024)?;

    let window = video_subsystem.window("OpenGGS Rust", 800, 600)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build()
        .map_err(|e| e.to_string())?;

    // Texture Creator must be created after canvas
    let texture_creator = canvas.texture_creator();

    // Load Assets
    let asset_path = "../base/c64/Player.png"; // Assuming running from rust-lib
    let player_texture = match AssetManager::load_texture(&texture_creator, asset_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to load texture {}: {}", asset_path, e);
            // Fallback or create a dummy texture if possible, or just panic for now
            return Err(format!("Failed to load asset: {}", e));
        }
    };
    
    // Try loading a sound (optional verify)
    let sound_path = "../base/audio/jump.wav";
    let jump_sound = match AssetManager::load_sound(sound_path) {
        Ok(s) => Some(s),
        Err(e) => {
            eprintln!("Failed to load sound {}: {}", sound_path, e);
            None
        }
    };

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump()?;
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                Event::KeyDown { keycode: Some(Keycode::Space), .. } => {
                     // Play sound
                     if let Some(sound) = &jump_sound {
                         let _ = sdl2::mixer::Channel::all().play(sound, 0);
                     }
                }
                _ => {}
            }
        }
        
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        
        // Draw Player Sprite
        // Just drawing the whole texture at 100, 100
        let query = player_texture.query();
        let dest_rect = Rect::new(100, 100, query.width, query.height);
        canvas.copy(&player_texture, None, dest_rect)?;

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }

    Ok(())
}