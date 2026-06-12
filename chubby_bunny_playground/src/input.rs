use chubby_bunny::FloatingPointNumber;
use core::time;
use itertools::Itertools;
use nalgebra::Vector2;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;
use wasm_bindgen::prelude::*;
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
    pub states: Vec<MouseState>,
}

#[derive(Debug, Clone, Copy)]
struct MouseState {
    pub mouse_position: Vector2<f32>,
    pub time_stamp: f32,
}
pub fn calc_average_mouse_speed_and_timestamp(
    states: &[MouseState],
    n: usize,
) -> Result<(Vector2<f32>, f32), &'static str> {
    if states.len() < 2 {
        return Err("Not enough values to calcualte speed");
    }
    let vals = states.iter().rev().take(n);
    let count = vals.len();
    Ok(vals
        .tuple_windows()
        .map(|(a, b)| {
            let delta_pos = b.mouse_position - a.mouse_position;
            let delta_time = (b.time_stamp - a.time_stamp) / 1000.0;
            (delta_pos / delta_time, delta_time)
        })
        .fold((Vector2::zeros(), 0_f32), |acc, delta| {
            (
                acc.0 + delta.0 / count as f32,
                acc.1 + delta.1 / count as f32,
            )
        }))
}

pub struct InputState {
    mouse_events: HashMap<MouseButton, Vec<MouseState>>,
    pub events: VecDeque<Event>,
}
impl InputState {
    pub fn new() -> Self {
        Self {
            mouse_events: HashMap::new(),
            events: VecDeque::new(),
        }
    }

    pub fn mouse_down(&mut self, button: MouseButton, position: Vector2<f32>, time_stamp: f32) {
        let new_events = vec![MouseState {
            mouse_position: position,
            time_stamp,
        }];

        self.mouse_events.insert(button, new_events.clone());
        self.events.push_back(Event {
            event_type: MouseEventType::Down,
            button,
            states: new_events,
        });
    }

    pub fn mouse_up(&mut self, button: MouseButton, position: Vector2<f32>, time_stamp: f32) {
        if let Some(mut events) = self.mouse_events.remove(&button) {
            let event = MouseState {
                mouse_position: position,
                time_stamp,
            };
            events.push(event);
            self.events.push_back(Event {
                event_type: MouseEventType::Up,
                button,
                states: events,
            });
        } else {
            eprint!("Received mouse up event for button {:?} without it being pressed first. This should only happen on init.", button);
        }
    }

    pub fn mouse_move(&mut self, position: Vector2<f32>, time_stamp: f32) {
        let new_event = MouseState {
            mouse_position: position,
            time_stamp: time_stamp,
        };
        for (button, events) in self.mouse_events.iter_mut() {
            events.push(new_event);
            self.events.push_back(Event {
                event_type: MouseEventType::Move,
                button: *button,
                states: events.clone(),
            });
        }
    }
}
