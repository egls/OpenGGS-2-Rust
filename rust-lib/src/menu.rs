use sdl2::pixels::Color;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::keyboard::Keycode;

use crate::gamestate::{GameState, StateTransition, Resources};
use crate::input::InputState;
use crate::text::BitmapFont;
use crate::game::Game;

pub struct MenuState {
    selected_option: usize,
    options: Vec<&'static str>,
}

impl MenuState {
    pub fn new() -> Self {
        Self { 
            selected_option: 0,
            options: vec!["START GAME", "OPTIONS", "CREDITS", "QUIT"],
        }
    }
}

impl GameState for MenuState {
    fn update(&mut self, input: &InputState) -> StateTransition {
        if input.quit_requested {
            return StateTransition::Quit;
        }

        if input.is_key_just_pressed(Keycode::Down) {
            self.selected_option = (self.selected_option + 1) % self.options.len();
        }
        if input.is_key_just_pressed(Keycode::Up) {
            if self.selected_option == 0 {
                self.selected_option = self.options.len() - 1;
            } else {
                self.selected_option -= 1;
            }
        }

        if input.is_key_just_pressed(Keycode::Return) {
            println!("Selected: {}", self.options[self.selected_option]);
            match self.selected_option {
                0 => return StateTransition::Swap(Box::new(Game::new())),
                3 => return StateTransition::Quit,
                _ => {}
            }
        }

        StateTransition::None
    }

    fn draw(&mut self, canvas: &mut Canvas<Window>, resources: &Resources) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(50, 50, 150)); // Blue background logic
        canvas.clear();

        let font = BitmapFont::new(resources.font_texture);
        
        let mut y = 200;
        for (i, option) in self.options.iter().enumerate() {
            let scale = if i == self.selected_option { 2.0 } else { 1.5 };
            let x = 300; // Center-ish
            
            // Draw selection indicator
            if i == self.selected_option {
                font.draw(canvas, x - 30, y, ">", scale)?;
            }
            
            font.draw(canvas, x, y, option, scale)?;
            y += 40;
        }

        Ok(())
    }
}
