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

use crate::tile_properties::TileSheetInfo;

pub struct Game {
    world: World,
    camera_x: f32,
    tile_info: TileSheetInfo,
    map: [[i32; 30]; 256],
}

impl Game {
    pub fn new(tile_info: TileSheetInfo) -> Self {
        let mut world = World::new();
        let mut map = [[0; 30]; 256];
        
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
                map = stage.array; // Store the map data
                
                // Iterate over array [x][y]
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
            tile_info,
            map,
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

        // --- 2. Physics & Collision System ---
        let map = &self.map;
        let tile_info = &self.tile_info;

        for (pos, vel, collider) in self.world.query_mut::<(&mut Position, &mut Velocity, &mut Collider)>().with::<&Player>() {
            // Apply Gravity
            vel.vy += GRAVITY;
            
            // Limit Fall Speed (Terminal Velocity)
            if vel.vy > TERMINAL_VELOCITY {
                vel.vy = TERMINAL_VELOCITY;
            }

            // Apply Friction
            if vel.vx > 0.0 {
                vel.vx -= FRICTION;
                if vel.vx < 0.0 { vel.vx = 0.0; }
            } else if vel.vx < 0.0 {
                vel.vx += FRICTION;
                if vel.vx > 0.0 { vel.vx = 0.0; }
            }

            collider.on_ground = false; // Reset ground state

            // --- X Axis Move & Collide ---
            pos.x += vel.vx;
            check_map_collision(map, tile_info, pos, vel, collider, true);

            // --- Y Axis Move & Collide ---
            pos.y += vel.vy;
            check_map_collision(map, tile_info, pos, vel, collider, false);
        }
        
        // --- 3. Camera System ---
        let mut player_x = 0.0;
        let mut found_player = false;
        
        for (pos, _player) in self.world.query::<(&Position, &Player)>().iter() {
            player_x = pos.x;
            found_player = true;
            break; 
        }
        
        if found_player {
             // Center player: camera_x = player_x - screen_width/2
             let target_cam_x = player_x - 350.0; // 800/2 roughly (actually 400, but let's shift it)
             // Smooth follow (Lerp)
             self.camera_x += (target_cam_x - self.camera_x) * 0.1;
             
             // Clamp to map bounds (0 to MapWidthPixels - ScreenWidth)
             // Map is 256 tiles * 16 = 4096 px. Screen is 800.
             if self.camera_x < 0.0 { self.camera_x = 0.0; }
             if self.camera_x > (4096.0 - 800.0) { self.camera_x = 4096.0 - 800.0; }
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

// Helper function for AABB Collision
fn check_map_collision(
    map: &[[i32; 30]; 256],
    tile_info: &TileSheetInfo,
    pos: &mut Position,
    vel: &mut Velocity,
    collider: &mut Collider,
    x_axis: bool
) {
    let check_x = pos.x;
    let check_y = pos.y;
    let width = collider.width;
    let height = collider.height;

    // Calculate tile range to check
    let left_tile = (check_x / 16.0).floor() as i32;
    let right_tile = ((check_x + width) / 16.0).floor() as i32;
    let top_tile = (check_y / 16.0).floor() as i32;
    let bottom_tile = ((check_y + height) / 16.0).floor() as i32;

    for tx in left_tile..=right_tile {
        for ty in top_tile..=bottom_tile {
            // Check bounds
            if tx < 0 || tx >= 256 || ty < 0 || ty >= 30 {
                continue;
            }

            let tile_id = map[tx as usize][ty as usize] as usize;
            
            // Check Solidity
            let is_solid = if tile_id < 2320 {
                tile_info.solid[tile_id] != 0
            } else {
                false
            };

            if is_solid {
                // Determine collision depth / side
                let tile_x = tx as f32 * 16.0;
                let tile_y = ty as f32 * 16.0;
                let tile_w = 16.0;
                let tile_h = 16.0;

                // AABB Check
                if check_x < tile_x + tile_w &&
                   check_x + width > tile_x &&
                   check_y < tile_y + tile_h &&
                   check_y + height > tile_y {
                    
                    if x_axis {
                        // Resolve X
                        if vel.vx > 0.0 { // Moving Right
                            pos.x = tile_x - width;
                            vel.vx = 0.0;
                        } else if vel.vx < 0.0 { // Moving Left
                            pos.x = tile_x + tile_w;
                            vel.vx = 0.0;
                        }
                    } else {
                        // Resolve Y
                        if vel.vy > 0.0 { // Falling
                            pos.y = tile_y - height;
                            vel.vy = 0.0;
                            collider.on_ground = true;
                        } else if vel.vy < 0.0 { // Jumping (Head bump)
                            pos.y = tile_y + tile_h;
                            vel.vy = 0.0;
                        }
                    }
                    return; // Resolve one collision per axis per frame (simple approach)
                }
            }
        }
    }
}
