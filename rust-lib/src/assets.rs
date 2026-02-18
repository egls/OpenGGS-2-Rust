use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;
use sdl2::image::LoadTexture;
use sdl2::mixer::{Chunk, Music};
use std::path::Path;

pub struct AssetManager {
    // In a real ECS, we might store handles here. 
    // For now, these are just helper functions or we could cache them.
}

impl AssetManager {
    pub fn load_texture<'a>(
        creator: &'a TextureCreator<WindowContext>,
        path: &str,
    ) -> Result<Texture<'a>, String> {
        creator.load_texture(Path::new(path))
    }

    pub fn load_sound(path: &str) -> Result<Chunk, String> {
        Chunk::from_file(Path::new(path))
    }
    
    pub fn load_music(path: &str) -> Result<Music, String> {
        Music::from_file(Path::new(path))
    }
}
