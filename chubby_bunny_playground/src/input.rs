use nalgebra::Vector2;
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
enum MouseButton {
    Left,
    Middle,
    Right,
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
enum MouseEventType {
    Down,
    Up,
    Move,
}

#[derive(Debug, Clone)]
struct Event {
    event_type: MouseEventType,
    button: MouseButton,
    states: Vec<MouseState>,
}

#[derive(Debug, Clone, Copy)]
struct MouseState {
    pub mouse_position: Vector2<f32>,
    pub time_stamp: Instant,
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

    pub fn mouse_down(&mut self, button: MouseButton, position: Vector2<f32>) {
        let new_events = vec![MouseState {
            mouse_position: position,
            time_stamp: Instant::now(),
        }];

        self.mouse_events.insert(button, new_events.clone());
        self.events.push_back(Event {
            event_type: MouseEventType::Down,
            button,
            states: new_events,
        });
    }

    pub fn mouse_up(&mut self, button: MouseButton, position: Vector2<f32>) {
        if let Some(mut events) = self.mouse_events.remove(&button) {
            let event = MouseState {
                mouse_position: position,
                time_stamp: Instant::now(),
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

    pub fn mouse_move(&mut self, button: MouseButton, position: Vector2<f32>) {
        let new_event = MouseState {
            mouse_position: position,
            time_stamp: Instant::now(),
        };
        for events in self.mouse_events.values_mut() {
            events.push(new_event);
            self.events.push_back(Event {
                event_type: MouseEventType::Move,
                button,
                states: events.clone(),
            });
        }
    }
}
