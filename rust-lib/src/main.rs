mod assets;
mod input;
mod gamestate;
mod menu;
mod text;
mod level;
mod game;
mod tile_properties;

use sdl2::pixels::Color;
use std::time::Duration;

use crate::assets::AssetManager;
use crate::input::InputState;
use crate::gamestate::{StateManager, GameState};

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

    let tiles_path = "../base/c64/Tiles.png";
    let tiles_texture = match AssetManager::load_texture(&texture_creator, tiles_path) {
        Ok(t) => t,
        Err(e) => return Err(format!("Failed to load tiles: {}", e)),
    };

    let player_path = "../base/c64/Player.png";
    let player_texture = match AssetManager::load_texture(&texture_creator, player_path) {
        Ok(t) => t,
        Err(e) => return Err(format!("Failed to load player: {}", e)),
    };

    let enemies_path = "../base/c64/Enemies.png";
    let enemies_texture = match AssetManager::load_texture(&texture_creator, enemies_path) {
        Ok(t) => t,
        Err(e) => return Err(format!("Failed to load enemies: {}", e)),
    };
    
    // Load Tile Properties
    let tile_props_path = "../base/Tilesheetinfo.tsi";
    let tile_info = match tile_properties::load_tile_properties(tile_props_path) {
        Ok(info) => info,
        Err(e) => return Err(format!("Failed to load tile properties: {}", e)),
    };
    println!("Loaded Tile Info: Tile Width={}, Height={}", tile_info.tile_width, tile_info.tile_height);
    
    // Test Level Loading
    let level_path = "../base/stages/classic.lvl";
    match level::load_stage(level_path, 1) { // Load Stage 1
        Ok(stage) => {
            let name = String::from_utf8_lossy(&stage.name);
            println!("Loaded Stage 1: {}", name);
            println!("Background Colour: {}", stage.background_colour);
        },
        Err(e) => eprintln!("Failed to load stage: {}", e),
    }

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    // Initialize Input and State
    let mut input_state = InputState::new();
    let mut state_manager = StateManager::new();    // Start with Menu State
    // We can't really pass tile_info to MenuState easily unless we change its sig.
    // For now, let's keep it in main and pass it when creating Game.
    // But StateManager holds Box<dyn GameState>.
    // To transition from Menu to Game, Menu needs to know about tile_info or be able to create Game.
    // Issue: MenuState doesn't have tile_info.
    // Solution:
    // 1. Pass tile_info into MenuState (requires changing MenuState).
    // 2. Or, for now, just clone it? but it's large (30KB).
    // Let's modify MenuState to hold tile_info so it can pass it to Game.
    // Start with Menu State
    state_manager.push(Box::new(menu::MenuState::new(tile_info)));
    
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
            tiles_texture: &tiles_texture,
            player_texture: &player_texture,
            enemies_texture: &enemies_texture,
        };
        state_manager.draw(&mut canvas, &resources)?;

        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }

    Ok(())
}