use itertools::Itertools;
use nalgebra::Vector2;
use std::collections::{HashMap, VecDeque};
use wasm_bindgen::prelude::*;

const MAX_STATES_PER_BUTTON: usize = 16;
#[wasm_bindgen]
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
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
}

#[derive(Debug, Copy, Clone)]
pub struct MouseState {
    pub mouse_position: Vector2<f32>,
    pub time_stamp: f32,
}

pub struct InputState {
    mouse_events: HashMap<MouseButton, VecDeque<MouseState>>,
    pub events: VecDeque<Event>,
}
impl InputState {
    pub fn new() -> Self {
        Self {
            mouse_events: HashMap::new(),
            events: VecDeque::new(),
        }
    }

    fn push_state_capped(states: &mut VecDeque<MouseState>, state: MouseState) {
        states.push_back(state);
        if states.len() > MAX_STATES_PER_BUTTON {
            states.pop_front();
        }
    }

    pub fn get_average_mouse_displacement_and_time_delta(
        &self,
        button: MouseButton,
        n: usize,
    ) -> Option<(Vector2<f32>, f32)> {
        let states = self.mouse_events.get(&button)?;
        if states.len() < 2 {
            return None;
        }
        let count = states.len().min(n);
        let vals = states.iter().rev().take(count);

        Some(
            vals.tuple_windows()
                .map(|(a, b)| {
                    let delta_pos = a.mouse_position - b.mouse_position;
                    let delta_time = (a.time_stamp - b.time_stamp) / 1000.0;
                    (delta_pos, delta_time)
                })
                .fold((Vector2::zeros(), 0_f32), |acc, delta| {
                    (
                        acc.0 + delta.0 / count as f32,
                        acc.1 + delta.1 / count as f32,
                    )
                }),
        )
    }
    pub fn mouse_down(&mut self, button: MouseButton, position: Vector2<f32>, time_stamp: f32) {
        let state = MouseState {
            mouse_position: position,
            time_stamp,
        };

        let mut new_events = VecDeque::new();
        Self::push_state_capped(&mut new_events, state);

        self.mouse_events.insert(button, new_events);
        self.events.push_back(Event {
            event_type: MouseEventType::Down,
            button,
            state,
        });
    }

    pub fn mouse_up(&mut self, button: MouseButton, position: Vector2<f32>, time_stamp: f32) {
        if let Some(mut states) = self.mouse_events.remove(&button) {
            let state = MouseState {
                mouse_position: position,
                time_stamp,
            };
            Self::push_state_capped(&mut states, state);
            self.events.push_back(Event {
                event_type: MouseEventType::Up,
                button,
                state,
            });
        } else {
            eprint!("Received mouse up event for button {:?} without it being pressed first. This should only happen on init.", button);
        }
    }

    pub fn mouse_move(&mut self, position: Vector2<f32>, time_stamp: f32) {
        let new_state = MouseState {
            mouse_position: position,
            time_stamp,
        };
        for (button, states) in self.mouse_events.iter_mut() {
            Self::push_state_capped(states, new_state);
            self.events.push_back(Event {
                event_type: MouseEventType::Move,
                button: *button,
                state: new_state,
            });
        }
    }
}
