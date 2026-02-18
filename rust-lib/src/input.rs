use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::collections::HashSet;

pub struct InputState {
    pub keys_pressed: HashSet<Keycode>,
    pub keys_just_pressed: HashSet<Keycode>,
    pub quit_requested: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_pressed: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            quit_requested: false,
        }
    }

    pub fn new_frame(&mut self) {
        self.keys_just_pressed.clear();
    }

    pub fn process_event(&mut self, event: &Event) {
        match event {
            Event::Quit { .. } => {
                self.quit_requested = true;
            }
            Event::KeyDown {
                keycode: Some(keycode),
                repeat: false,
                ..
            } => {
                self.keys_pressed.insert(*keycode);
                self.keys_just_pressed.insert(*keycode);
                
                // Hardcoded quit on escape for now, or handle in game logic
                if *keycode == Keycode::Escape {
                    self.quit_requested = true;
                }
            }
            Event::KeyUp {
                keycode: Some(keycode),
                ..
            } => {
                self.keys_pressed.remove(keycode);
            }
            _ => {}
        }
    }

    pub fn is_key_down(&self, key: Keycode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn is_key_just_pressed(&self, key: Keycode) -> bool {
        self.keys_just_pressed.contains(&key)
    }
}
