use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;
use crate::input::InputState;

pub struct Resources<'a> {
    pub font_texture: &'a Texture<'a>,
    pub tiles_texture: &'a Texture<'a>,
}

pub enum StateTransition {
    None,
    Push(Box<dyn GameState>),
    Pop,
    Quit,
    Swap(Box<dyn GameState>), 
}

pub trait GameState {
    fn update(&mut self, input: &InputState) -> StateTransition;
    fn draw(&mut self, canvas: &mut Canvas<Window>, resources: &Resources) -> Result<(), String>;
}

pub struct StateManager {
    states: Vec<Box<dyn GameState>>,
}

impl StateManager {
    pub fn new() -> Self {
        Self { states: Vec::new() }
    }

    pub fn push(&mut self, state: Box<dyn GameState>) {
        self.states.push(state);
    }

    pub fn pop(&mut self) {
        self.states.pop();
    }

    pub fn update(&mut self, input: &InputState) -> bool {
        if let Some(state) = self.states.last_mut() {
            match state.update(input) {
                StateTransition::None => true,
                StateTransition::Pop => {
                    self.pop();
                    !self.states.is_empty()
                }
                StateTransition::Push(new_state) => {
                    self.push(new_state);
                    true
                }
                StateTransition::Swap(new_state) => {
                    self.pop();
                    self.push(new_state);
                    true
                }
                StateTransition::Quit => false,
            }
        } else {
            false
        }
    }

    pub fn draw(&mut self, canvas: &mut Canvas<Window>, resources: &Resources) -> Result<(), String> {
        if let Some(state) = self.states.last_mut() {
            state.draw(canvas, resources)
        } else {
            Ok(())
        }
    }
}
