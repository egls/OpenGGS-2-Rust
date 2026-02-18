use hecs::World;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::rect::Rect;
use sdl2::keyboard::Keycode;
use sdl2::mixer::Chunk;

use crate::gamestate::{GameState, StateTransition, Resources};
use crate::input::InputState;
use crate::level;
use crate::text::BitmapFont;

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

// --- Animation ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerDirection {
    Right,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerStance {
    Stand,
    Walk1,
    Walk2,
    Jump,
}

#[derive(Debug, Clone, Copy)]
pub struct Animation {
    pub direction: PlayerDirection,
    pub stance: PlayerStance,
    pub walk_timer: u32,
}

// Frame data: (src_x, src_y, src_w, src_h)
type Frame = (i32, i32, u32, u32);

// --- Enemy ---

pub struct Enemy {
    pub enemy_type: u32,    // 1–15
    pub alive: bool,
    pub direction: i32,     // 0 = left, 1 = right (matches C++ NPC_LEFT/NPC_RIGHT)
    pub anim_frame: usize,  // 0–3
    pub anim_timer: u32,
}

#[derive(Clone)]
struct EnemyFrameData {
    col_w: f32,
    col_h: f32,
    frames_right: [Frame; 4],
    frames_left:  [Frame; 4],
    frame_dead:   Frame,
}

use crate::tile_properties::TileSheetInfo;

fn load_chunk(path: &str) -> Option<Chunk> {
    match Chunk::from_file(path) {
        Ok(c) => Some(c),
        Err(e) => { eprintln!("Audio: failed to load {}: {}", path, e); None }
    }
}

pub struct Game {
    world: World,
    camera_x: f32,
    tile_info: TileSheetInfo,
    map: [[i32; 30]; 256],
    player_frames: Vec<Frame>,
    enemy_frames: Vec<EnemyFrameData>,
    stomp_bounce: bool,
    // HUD state
    score: u32,
    lives: u32,
    stage_num: u32,
    // Audio
    snd_jump:  Option<Chunk>,
    snd_stomp: Option<Chunk>,
}

impl Game {
    pub fn new(tile_info: TileSheetInfo) -> Self {
        let mut world = World::new();
        let mut map = [[0; 30]; 256];
        
        // Parse Player.txt frame data (92 ints = 23 frames × 4 values)
        let player_frames = Self::load_player_frames("../base/c64/Player.txt");
        
        // Parse Enemies.txt frame data (690 ints = 15 types × 46 ints each)
        let enemy_frames = Self::load_enemy_frames("../base/c64/Enemies.txt");
        
        // Spawn Player — position updated below once level is loaded
        let player_entity = world.spawn((
            Player,
            Position { x: 100.0, y: 300.0 },
            Velocity { vx: 0.0, vy: 0.0 },
            Collider { width: 28.0, height: 42.0, on_ground: false },
            Animation {
                direction: PlayerDirection::Right,
                stance: PlayerStance::Stand,
                walk_timer: 0,
            },
        ));

        // Load Level 1
        let level_path = "../base/stages/classic.lvl";
        match level::load_stage(level_path, 1) {
            Ok(stage) => {
                println!("Game: Loaded Stage 1");
                map = stage.array;

                // Use level start position for player (player 1 = index 0)
                let sx = stage.start_position_x[0] as f32;
                let sy = stage.start_position_y[0] as f32;
                if sx > 0.0 || sy > 0.0 {
                    if let Ok(mut pos) = world.get::<&mut Position>(player_entity) {
                        pos.x = sx;
                        pos.y = sy;
                    }
                }
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

                // Spawn enemies from level data
                const MAX_ENEMIES: usize = 50;
                for i in 0..MAX_ENEMIES {
                    if stage.enemy_in_use[i] != 0 {
                        let etype = stage.enemy_type[i] as u32;
                        let (col_w, col_h) = if etype >= 1 && (etype as usize - 1) < enemy_frames.len() {
                            let ef = &enemy_frames[etype as usize - 1];
                            (ef.col_w, ef.col_h)
                        } else {
                            (32.0, 32.0)
                        };
                        world.spawn((
                            Enemy {
                                enemy_type: etype,
                                alive: true,
                                direction: stage.enemy_direction[i],
                                anim_frame: 0,
                                anim_timer: 0,
                            },
                            Position {
                                x: stage.enemy_pos_x[i] as f32,
                                y: stage.enemy_pos_y[i] as f32,
                            },
                            Velocity { vx: 0.0, vy: 0.0 },
                            Collider { width: col_w, height: col_h, on_ground: false },
                        ));
                    }
                }
                println!("Game: Spawned enemies from level data");
            },
            Err(e) => eprintln!("Game: Failed to load stage: {}", e),
        }

        // Load audio
        let snd_jump  = load_chunk("../base/audio/jump.wav");
        let snd_stomp = load_chunk("../base/audio/stomp.wav");

        Self {
            world,
            camera_x: 0.0,
            tile_info,
            map,
            player_frames,
            enemy_frames,
            stomp_bounce: false,
            score: 0,
            lives: 3,
            stage_num: 1,
            snd_jump,
            snd_stomp,
        }
    }

    fn load_player_frames(path: &str) -> Vec<Frame> {
        let mut frames = Vec::new();
        let content = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to load player frames: {}", e);
                return frames;
            }
        };
        
        // Parse all comma-separated integers
        let nums: Vec<i32> = content
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.trim().parse::<i32>().ok())
            .collect();
        
        // Group into frames of 4: (x, y, w, h)
        for chunk in nums.chunks(4) {
            if chunk.len() == 4 {
                frames.push((chunk[0], chunk[1], chunk[2] as u32, chunk[3] as u32));
            }
        }
        
        println!("Loaded {} player frames", frames.len());
        frames
    }

    fn load_enemy_frames(path: &str) -> Vec<EnemyFrameData> {
        let mut result = Vec::new();
        let content = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => { eprintln!("Failed to load enemy frames: {}", e); return result; }
        };
        let nums: Vec<i32> = content
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.trim().parse::<i32>().ok())
            .collect();

        // Each enemy type: 46 ints
        // [0] = ColW, [1] = ColH
        // [2..18]  = 4 right frames × 4 (x,y,w,h)
        // [18..22] = dead frame (x,y,w,h)
        // [22..38] = 4 left frames × 4
        // [38..46] = 8 extra ints (skipped)
        for chunk in nums.chunks(46) {
            if chunk.len() < 38 { break; }
            let col_w = chunk[0] as f32;
            let col_h = chunk[1] as f32;
            let mut frames_right = [(0i32,0i32,0u32,0u32); 4];
            let mut frames_left  = [(0i32,0i32,0u32,0u32); 4];
            for i in 0..4 {
                let base = 2 + i * 4;
                frames_right[i] = (chunk[base], chunk[base+1], chunk[base+2] as u32, chunk[base+3] as u32);
            }
            let frame_dead = (chunk[18], chunk[19], chunk[20] as u32, chunk[21] as u32);
            for i in 0..4 {
                let base = 22 + i * 4;
                frames_left[i] = (chunk[base], chunk[base+1], chunk[base+2] as u32, chunk[base+3] as u32);
            }
            result.push(EnemyFrameData { col_w, col_h, frames_right, frames_left, frame_dead });
        }
        println!("Loaded {} enemy types", result.len());
        result
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

        // --- 1. Input System ---
        // (Input is applied inside the physics loop below)

        // --- 2. Physics & Collision System ---
        let map = &self.map;
        let tile_info = &self.tile_info;

        for (pos, vel, collider, anim) in self.world.query_mut::<(&mut Position, &mut Velocity, &mut Collider, &mut Animation)>().with::<&Player>() {
            // Apply stomp bounce from previous frame
            if self.stomp_bounce {
                vel.vy = -8.0;
                self.stomp_bounce = false;
            }

            // --- Input ---
            let pressing_left  = input.keys_pressed.contains(&Keycode::Left);
            let pressing_right = input.keys_pressed.contains(&Keycode::Right);

            if pressing_right {
                vel.vx += ACCELERATION;
                if vel.vx > MAX_RUN_SPEED { vel.vx = MAX_RUN_SPEED; }
                anim.direction = PlayerDirection::Right;
            }
            if pressing_left {
                vel.vx -= ACCELERATION;
                if vel.vx < -MAX_RUN_SPEED { vel.vx = -MAX_RUN_SPEED; }
                anim.direction = PlayerDirection::Left;
            }
            if input.keys_pressed.contains(&Keycode::Space) && collider.on_ground {
                vel.vy = -JUMP_STRENGTH;
                // Play jump sound
                if let Some(ref chunk) = self.snd_jump {
                    let _ = sdl2::mixer::Channel::all().play(chunk, 0);
                }
            }

            // Apply Gravity
            vel.vy += GRAVITY;
            if vel.vy > TERMINAL_VELOCITY { vel.vy = TERMINAL_VELOCITY; }

            // Apply Friction only when no key is held (decelerate to stop)
            if !pressing_left && !pressing_right {
                if vel.vx > 0.0 {
                    vel.vx -= FRICTION;
                    if vel.vx < 0.0 { vel.vx = 0.0; }
                } else if vel.vx < 0.0 {
                    vel.vx += FRICTION;
                    if vel.vx > 0.0 { vel.vx = 0.0; }
                }
            }

            collider.on_ground = false;

            // --- X Axis Move & Collide ---
            pos.x += vel.vx;
            check_map_collision(map, tile_info, pos, vel, collider, true);

            // --- Y Axis Move & Collide ---
            pos.y += vel.vy;
            check_map_collision(map, tile_info, pos, vel, collider, false);

            // --- Animation State Machine ---
            if !collider.on_ground {
                anim.stance = PlayerStance::Jump;
            } else if vel.vx.abs() > 0.1 {
                anim.walk_timer += 1;
                if anim.walk_timer >= 16 { anim.walk_timer = 0; }
                anim.stance = if anim.walk_timer < 8 { PlayerStance::Walk1 } else { PlayerStance::Walk2 };
            } else {
                anim.stance = PlayerStance::Stand;
                anim.walk_timer = 0;
            }
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
             let target_cam_x = player_x - 350.0;
             // Smooth follow (Lerp)
             self.camera_x += (target_cam_x - self.camera_x) * 0.1;
             // Clamp to map bounds
             if self.camera_x < 0.0 { self.camera_x = 0.0; }
             if self.camera_x > (4096.0 - 800.0) { self.camera_x = 4096.0 - 800.0; }
        }

        // --- 4. Enemy AI System ---
        const ENEMY_SPEED: f32 = 2.5;
        let map = &self.map;
        let tile_info = &self.tile_info;

        for (pos, vel, collider, enemy) in self.world.query_mut::<(&mut Position, &mut Velocity, &mut Collider, &mut Enemy)>() {
            if !enemy.alive { continue; }

            // Gravity
            vel.vy += GRAVITY;
            if vel.vy > TERMINAL_VELOCITY { vel.vy = TERMINAL_VELOCITY; }

            // Horizontal walk: direction 1 = right, 0 = left
            vel.vx = if enemy.direction == 1 { ENEMY_SPEED } else { -ENEMY_SPEED };

            collider.on_ground = false;

            // X axis: move and check wall collision
            pos.x += vel.vx;
            let old_vx = vel.vx;
            check_map_collision(map, tile_info, pos, vel, collider, true);
            // If vx was zeroed by collision, reverse direction
            if old_vx != 0.0 && vel.vx == 0.0 {
                enemy.direction = if enemy.direction == 1 { 0 } else { 1 };
            }

            // Y axis
            pos.y += vel.vy;
            check_map_collision(map, tile_info, pos, vel, collider, false);

            // Animation: cycle 0-3 every 8 frames
            enemy.anim_timer += 1;
            if enemy.anim_timer >= 8 {
                enemy.anim_timer = 0;
                enemy.anim_frame = (enemy.anim_frame + 1) % 4;
            }
        }

        // --- 5. Player-Enemy Stomp Collision ---
        // Collect player state
        let mut player_pos_x = 0.0f32;
        let mut player_pos_y = 0.0f32;
        let mut player_w = 28.0f32;
        let mut player_h = 42.0f32;
        let mut player_vy = 0.0f32;
        let mut player_entity = None;

        let mut player_query = self.world.query::<(hecs::Entity, &Position, &Collider, &Velocity, &Player)>();
        for (entity, pos, collider, vel, _player) in player_query.iter() {
            player_pos_x = pos.x;
            player_pos_y = pos.y;
            player_w = collider.width;
            player_h = collider.height;
            player_vy = vel.vy;
            player_entity = Some(entity);
            break;
        }

        if let Some(_player_ent) = player_entity {
            let mut stomp_targets = Vec::new();

            {
                let mut enemy_query = self.world.query::<(hecs::Entity, &Position, &Collider, &Enemy)>();
                for (entity, pos, collider, enemy) in enemy_query.iter() {
                    if !enemy.alive { continue; }

                    let px2 = player_pos_x + player_w;
                    let py2 = player_pos_y + player_h;
                    let ex1 = pos.x;
                    let ex2 = pos.x + collider.width;
                    let ey1 = pos.y;
                    let ey2 = pos.y + collider.height;

                    if player_pos_x < ex2 && px2 > ex1 && player_pos_y < ey2 && py2 > ey1 {
                        if player_vy > 0.0 && py2 <= ey1 + 8.0 {
                            stomp_targets.push(entity);
                        }
                        // side collision: could reduce health here later
                    }
                }
            } // enemy_query dropped

            // Kill stomped enemies and set bounce flag (applied next player physics tick)
            if !stomp_targets.is_empty() {
                self.stomp_bounce = true;
                self.score += 100 * stomp_targets.len() as u32;
                // Play stomp sound
                if let Some(ref chunk) = self.snd_stomp {
                    let _ = sdl2::mixer::Channel::all().play(chunk, 0);
                }
                for entity in stomp_targets {
                    if let Ok(mut enemy) = self.world.get::<&mut Enemy>(entity) {
                        enemy.alive = false;
                    }
                }
            }
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
        for (pos, _collider, anim, _player) in self.world.query::<(&Position, &Collider, &Animation, &Player)>().iter() {
            let screen_x = pos.x - self.camera_x;
            
            let dir_offset = match anim.direction {
                PlayerDirection::Right => 0,
                PlayerDirection::Left  => 5,
            };
            let stance_offset = match anim.stance {
                PlayerStance::Stand => 0,
                PlayerStance::Walk1 => 1,
                PlayerStance::Walk2 => 2,
                PlayerStance::Jump  => 3,
            };
            let frame_idx = dir_offset + stance_offset;
            
            let (sx, sy, sw, sh) = if frame_idx < self.player_frames.len() {
                self.player_frames[frame_idx]
            } else {
                (0, 0, 32, 44) // fallback
            };
            
            let src  = Rect::new(sx, sy, sw, sh);
            let dest = Rect::new(screen_x as i32, pos.y as i32, sw, sh);
            
            canvas.copy(resources.player_texture, src, dest)?;
        }

        // Draw Enemies
        for (pos, enemy) in self.world.query::<(&Position, &Enemy)>().iter() {
            let screen_x = pos.x - self.camera_x;
            // Cull off-screen enemies
            if screen_x < -64.0 || screen_x > 864.0 { continue; }

            let etype_idx = (enemy.enemy_type as usize).saturating_sub(1);
            if etype_idx >= self.enemy_frames.len() { continue; }
            let fd = &self.enemy_frames[etype_idx];

            let (sx, sy, sw, sh) = if !enemy.alive {
                fd.frame_dead
            } else if enemy.direction == 1 {
                fd.frames_right[enemy.anim_frame % 4]
            } else {
                fd.frames_left[enemy.anim_frame % 4]
            };

            if sw == 0 || sh == 0 { continue; } // skip zero-size frames

            let src  = Rect::new(sx, sy, sw, sh);
            let dest = Rect::new(screen_x as i32, pos.y as i32, sw, sh);
            canvas.copy(resources.enemies_texture, src, dest)?;
        }

        // Draw HUD (on top of everything)
        {
            let font = BitmapFont::new(resources.font_texture);
            let scale = 1.5;

            // PLAYER / score
            font.draw(canvas, 10, 4, "PLAYER", scale)?;
            font.draw(canvas, 10, 20, &self.score.to_string(), scale)?;

            // LIVES
            font.draw(canvas, 320, 4, "LIVES", scale)?;
            font.draw(canvas, 320, 20, &self.lives.to_string(), scale)?;

            // STAGE
            font.draw(canvas, 630, 4, "STAGE", scale)?;
            font.draw(canvas, 630, 20, &self.stage_num.to_string(), scale)?;
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
