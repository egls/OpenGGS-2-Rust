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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Player;

#[derive(Debug, Clone, Copy)]
pub struct Velocity {
    pub vx: f32,
    pub vy: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct Collider {
    pub width: f32,
    pub height: f32,
    pub on_ground: bool,
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
        
        // Spawn Player
        world.spawn((
            Player,
            Position { x: 100.0, y: 300.0 }, // Starting pos from C++ PC_Define
            Velocity { vx: 0.0, vy: 0.0 },
            Collider { width: 28.0, height: 42.0, on_ground: false },
        ));

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
        
        // --- Physics Constants ---
        const GRAVITY: f32 = 1.0;
        const TERMINAL_VELOCITY: f32 = 15.0;
        const FRICTION: f32 = 1.0;
        const MAX_RUN_SPEED: f32 = 4.0; 
        const ACCELERATION: f32 = 1.0;
        const JUMP_STRENGTH: f32 = 12.0; // Reduced from 24 temporarily for testing

        // --- 1. Input System (Player only) ---
        for (vel, collider) in self.world.query_mut::<(&mut Velocity, &Collider)>().with::<&Player>() {
            if input.is_key_down(Keycode::Left) {
                vel.vx -= ACCELERATION;
            }
            if input.is_key_down(Keycode::Right) {
                vel.vx += ACCELERATION;
            }
            // Jump
            if input.is_key_just_pressed(Keycode::Space) && collider.on_ground {
                vel.vy = -JUMP_STRENGTH;
            }
        }

        // --- 2. Physics Integration System ---
        for (pos, vel, collider) in self.world.query_mut::<(&mut Position, &mut Velocity, &mut Collider)>().with::<&Player>() {
            // Apply Gravity
            vel.vy += GRAVITY;
            
            // Clamp Velocity
            if vel.vy > TERMINAL_VELOCITY { vel.vy = TERMINAL_VELOCITY; }
            if vel.vx > MAX_RUN_SPEED { vel.vx = MAX_RUN_SPEED; }
            if vel.vx < -MAX_RUN_SPEED { vel.vx = -MAX_RUN_SPEED; }

            // Apply Friction (if not accelerating? simplified for now)
            if !input.is_key_down(Keycode::Left) && !input.is_key_down(Keycode::Right) {
                 if vel.vx > 0.0 { vel.vx -= FRICTION; if vel.vx < 0.0 { vel.vx = 0.0; } }
                 if vel.vx < 0.0 { vel.vx += FRICTION; if vel.vx > 0.0 { vel.vx = 0.0; } }
            }

            // Apply Velocity to Position (Naive integration)
            pos.x += vel.vx;
            pos.y += vel.vy;
            
            // Camera follow player
             // We can't access self.camera_x inside this loop easily if we want to write to it.
             // We'll update camera after loop.
        }
        
        // --- 3. Collision System (Naive floor check) ---
        // TODO: AABB with Tiles. For now, just a floor plane at y=400 for testing gravity
        for (pos, vel, collider) in self.world.query_mut::<(&mut Position, &mut Velocity, &mut Collider)>().with::<&Player>() {
             collider.on_ground = false; // Reset
             
             // Simple floor collision test
             if pos.y + collider.height > 400.0 {
                 pos.y = 400.0 - collider.height;
                 vel.vy = 0.0;
                 collider.on_ground = true;
             }
        }
        
        // Update Camera (Post-Physics)
        let mut player_x = 0.0;
        let mut found_player = false;
        for (pos, _player) in self.world.query::<(&Position, &Player)>().iter() {
            player_x = pos.x;
            found_player = true;
            break;
        }
        if found_player {
             // Center player: camera_x = player_x - screen_width/2
             let target_cam_x = player_x - 320.0; // 640/2
             // Smooth follow or direct? Direct for now
             self.camera_x = target_cam_x;
             if self.camera_x < 0.0 { self.camera_x = 0.0; }
        }

        StateTransition::None
    }

    fn draw(&mut self, canvas: &mut Canvas<Window>, resources: &Resources) -> Result<(), String> {
        canvas.set_draw_color(sdl2::pixels::Color::RGB(100, 100, 255)); // Sky blue background
        canvas.clear();

        let tile_width = 16;
        let tile_height = 16;
        let tiles_per_row = 40; 

        // Draw Tiles
        for (pos, tile) in self.world.query_mut::<(&Position, &Tile)>() {
            // Simple culling
            let screen_x = pos.x - self.camera_x;
            if screen_x < -16.0 || screen_x > 800.0 {
                continue;
            }
            
            let src_x = (tile.tile_id % tiles_per_row) * tile_width;
            let src_y = (tile.tile_id / tiles_per_row) * tile_height;
            
            let src = Rect::new(src_x as i32, src_y as i32, tile_width as u32, tile_height as u32);
            let dest = Rect::new(screen_x as i32, pos.y as i32, tile_width as u32, tile_height as u32);
            
            canvas.copy(resources.tiles_texture, src, dest)?;
        }
        
        // Draw Player
         for (pos, collider, _player) in self.world.query::<(&Position, &Collider, &Player)>().iter() {
             let screen_x = pos.x - self.camera_x;
             // Draw simple sprite rect for now (assuming frame 0 of player texture)
             // Using entire player texture or a specific frame? 
             // PC definition had Frames. Let's just draw a subset of the texture.
             // Player.png is 7312 bytes.
             // Let's guess default frame is top-left.
             let src = Rect::new(0, 0, 50, 50); // Guessing sprite size from texture
             // Actually, collider says 28x42.
             let dest = Rect::new(screen_x as i32, pos.y as i32, collider.width as u32, collider.height as u32);
             
             canvas.copy(resources.player_texture, src, dest)?;
         }

        Ok(())
    }
}
