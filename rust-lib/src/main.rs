mod assets;
mod input;
mod gamestate;
mod menu;
mod text;

use sdl2::pixels::Color;
use std::time::Duration;

use crate::assets::AssetManager;
use crate::input::InputState;
use crate::gamestate::{StateManager, GameState};
use crate::menu::MenuState;

pub fn main() -> Result<(), String> {
    env_logger::init();
    
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let _image_context = sdl2::image::init(sdl2::image::InitFlag::PNG | sdl2::image::InitFlag::JPG)?;
    let _audio_subsystem = sdl_context.audio()?;
    let _mixer_context = sdl2::mixer::init(sdl2::mixer::InitFlag::OGG | sdl2::mixer::InitFlag::MP3)?;

    sdl2::mixer::open_audio(44100, sdl2::mixer::AUDIO_S16LSB, 2, 1024)?;

    let window = video_subsystem.window("OpenGGS Rust", 800, 600)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build()
        .map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();
    // Load Assets
    let asset_path = "../base/c64/Player.png"; 
    let _player_texture = match AssetManager::load_texture(&texture_creator, asset_path) {
        Ok(t) => t,
        Err(e) => return Err(format!("Failed to load asset: {}", e)),
    };
    
    let font_path = "../base/Font.png";
    let font_texture = match AssetManager::load_texture(&texture_creator, font_path) {
        Ok(t) => t,
        Err(e) => return Err(format!("Failed to load font: {}", e)),
    };
    
    // Sound loading omitted for brevity, logic remains valid if kept

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    // Initialize Input and State
    let mut input_state = InputState::new();
    let mut state_manager = StateManager::new();
    state_manager.push(Box::new(MenuState::new()));
    
    let mut event_pump = sdl_context.event_pump()?;
    
    'running: loop {
        input_state.new_frame();
        
        for event in event_pump.poll_iter() {
            input_state.process_event(&event);
        }

        if input_state.quit_requested {
            break 'running;
        }
        
        // Update State
        if !state_manager.update(&input_state) {
            break 'running;
        }

        // Draw State
        let resources = crate::gamestate::Resources {
            font_texture: &font_texture,
        };
        state_manager.draw(&mut canvas, &resources)?;

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }

    Ok(())
}