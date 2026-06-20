use nalgebra::Vector2;
use std::collections::VecDeque;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    None,
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum MouseEventType {
    Down,
    Up,
    Move,
}

#[derive(Debug, Clone)]
pub struct Event {
    pub event_type: MouseEventType,
    pub button: MouseButton,
    pub state: MouseState,
    pub last_state: Option<MouseState>,
}

#[derive(Debug, Copy, Clone)]
pub struct MouseState {
    pub mouse_position: Vector2<f32>,
    pub time_stamp: f32,
}

pub struct InputState {
    pub events: VecDeque<Event>,
    last_mouse_state: Option<MouseState>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            last_mouse_state: None,
        }
    }

    pub fn mouse_down(&mut self, button: MouseButton, position: Vector2<f32>, time_stamp: f32) {
        let state = MouseState {
            mouse_position: position,
            time_stamp,
        };

        self.events.push_back(Event {
            event_type: MouseEventType::Down,
            button,
            state,
            last_state: self.last_mouse_state,
        });
        self.last_mouse_state = Some(state);
    }

    pub fn mouse_up(&mut self, button: MouseButton, position: Vector2<f32>, time_stamp: f32) {
        let state = MouseState {
            mouse_position: position,
            time_stamp,
        };
        self.events.push_back(Event {
            event_type: MouseEventType::Up,
            button,
            state,
            last_state: self.last_mouse_state,
        });
        self.last_mouse_state = Some(state);
    }

    pub fn mouse_move(&mut self, position: Vector2<f32>, time_stamp: f32) {
        let new_state = MouseState {
            mouse_position: position,
            time_stamp,
        };
        self.events.push_back(Event {
            event_type: MouseEventType::Move,
            button: MouseButton::None,
            state: new_state,
            last_state: self.last_mouse_state,
        });
        self.last_mouse_state = Some(new_state);
    }
}
