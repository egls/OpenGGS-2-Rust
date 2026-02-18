use hecs::World;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::rect::Rect;
use sdl2::keyboard::Keycode;

use crate::gamestate::{GameState, StateTransition, Resources};
use crate::input::InputState;
use crate::level;

pub struct Position {
    pub x: f32,
    pub y: f32,
}

pub struct Tile {
    pub tile_id: i32,
}

pub struct Game {
    world: World,
    camera_x: f32,
}

impl Game {
    pub fn new() -> Self {
        let mut world = World::new();
        
        // Load Level 1
        let level_path = "../base/stages/classic.lvl";
        match level::load_stage(level_path, 1) { // Load Stage 1
            Ok(stage) => {
                println!("Game: Loaded Stage 1");
                // Iterate over array [x][y]
                // array is [[i32; 30]; 256]
                for x in 0..256 {
                    for y in 0..30 {
                        let tile_id = stage.array[x][y];
                        if tile_id != 0 {
                            world.spawn((
                                Position {
                                    x: (x * 16) as f32,
                                    y: (y * 16) as f32,
                                },
                                Tile { tile_id },
                            ));
                        }
                    }
                }
            },
            Err(e) => eprintln!("Game: Failed to load stage: {}", e),
        }

        Self {
            world,
            camera_x: 0.0,
        }
    }
}

impl GameState for Game {
    fn update(&mut self, input: &InputState) -> StateTransition {
        if input.quit_requested {
            return StateTransition::Quit;
        }
        
        if input.is_key_down(Keycode::Left) {
            self.camera_x -= 4.0;
        }
        if input.is_key_down(Keycode::Right) {
            self.camera_x += 4.0;
        }
        
        // Clamp camera
        // Map width = 256 * 16 = 4096
        // Screen width = 800 (or 640?)
        // Let's assume 800 for now.
        if self.camera_x < 0.0 { self.camera_x = 0.0; }
        // if self.camera_x > ... 

        StateTransition::None
    }

    fn draw(&mut self, canvas: &mut Canvas<Window>, resources: &Resources) -> Result<(), String> {
        canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
        canvas.clear();

        let tile_width = 16;
        let tile_height = 16;
        let tiles_per_row = 40; 

        for (pos, tile) in self.world.query_mut::<(&Position, &Tile)>() {
            // Simple culling
            let screen_x = pos.x - self.camera_x;
            // ...
            
            let src_x = (tile.tile_id % tiles_per_row) * tile_width;
            let src_y = (tile.tile_id / tiles_per_row) * tile_height;
            
            let src = Rect::new(src_x as i32, src_y as i32, tile_width as u32, tile_height as u32);
            let dest = Rect::new(screen_x as i32, pos.y as i32, tile_width as u32, tile_height as u32);
            
            canvas.copy(resources.tiles_texture, src, dest)?;
        }

        Ok(())
    }
}
