use nalgebra::Vector2;
use std::collections::VecDeque;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
/// Represents the mouse buttons that can be interacted with.
pub enum MouseButton {
    Left,
    Middle,
    Right,
    None,
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
/// Represents the type of mouse event that occurred, such as a button press, release, or movement.
pub enum MouseEventType {
    Down,
    Up,
    Move,
}

#[derive(Debug, Clone)]
/// Represents a user input event, including the type of event, which mouse button was involved, the current state of the mouse, and optionally the previous state of the mouse for movement events.
pub struct Event {
    pub event_type: MouseEventType,
    pub button: MouseButton,
    pub state: MouseState,
    pub last_state: Option<MouseState>,
}

#[derive(Debug, Copy, Clone)]
/// Represents the state of the mouse at a given point in time,
/// including its position and the timestamp of the event.
pub struct MouseState {
    pub mouse_position: Vector2<f32>,
    pub time_stamp: f32,
}

#[derive(Debug, Clone)]
/// Represents the state of the user input,
/// this contains all events that have occurred since the last update
pub struct InputState {
    pub events: VecDeque<Event>,
    last_mouse_state: Option<MouseState>,
}

impl InputState {
    /// Creates a new InputState with an empty event queue and no last mouse state.
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            last_mouse_state: None,
        }
    }

    /// Records a mouse down event in the input state.
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

    /// Records a mouse up event in the input state.
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

    /// Records a mouse move event in the input state.
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
