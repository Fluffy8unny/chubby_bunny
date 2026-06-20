use crate::input::{Event, InputState, MouseButton};
use crate::js_types::{bodies_to_polygon_arrays, OutgoingEvent, PolygonArray};
use chubby_bunny_core::{Body, BodyId};

use chubby_bunny_svg::BodyMeta;
use nalgebra::Vector2;
use std::collections::{HashMap, VecDeque};
use wasm_bindgen::prelude::*;
pub struct GameLoop<G: Game> {
    pub game_impl: Box<G>,
    pub polygon_arrays: Vec<PolygonArray>,
    pub user_input: InputState,
}

pub trait Game {
    fn init(&mut self, width: usize, height: usize);
    fn reset(&mut self, width: f32, height: f32);
    fn update(
        &mut self,
        incoming_events: VecDeque<Event>,
        mouse_speed: Option<Vector2<f32>>,
        dt_ms: f32,
    ) -> Vec<OutgoingEvent>;
    fn bodies_to_render(&self) -> &[Body];
    fn meta_data_to_render(&self) -> &HashMap<BodyId, BodyMeta>;
    fn current_selection_to_render(&self) -> &[BodyId];
}

impl<G: Game> GameLoop<G> {
    pub fn init(&mut self, width: usize, height: usize) {
        self.polygon_arrays.clear();
        self.user_input = InputState::new();
        self.game_impl.init(width, height);
    }
    pub fn update(&mut self, dt_ms: f32) -> Result<JsValue, JsValue> {
        let avg_mouse_speed = self
            .user_input
            .get_average_mouse_displacement_and_time_delta(MouseButton::Left, 5)
            .map(|(displacement, _)| displacement);
        let outgoing_events = self.game_impl.update(
            self.user_input.events.drain(..).collect(),
            avg_mouse_speed,
            dt_ms,
        );
        self.polygon_arrays = bodies_to_polygon_arrays(
            self.game_impl.bodies_to_render().iter(),
            self.game_impl.meta_data_to_render(),
            self.game_impl.current_selection_to_render(),
        );
        serde_wasm_bindgen::to_value(&outgoing_events)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn reset(&mut self, width: f32, height: f32) {
        self.polygon_arrays.clear();
        self.game_impl.reset(width, height);
    }

    pub fn get_polygon_arrays(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.polygon_arrays)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    pub fn mouse_down(&mut self, x: f32, y: f32, mouse_button: MouseButton, time_stamp: f32) {
        self.user_input
            .mouse_down(mouse_button, Vector2::new(x, y), time_stamp);
    }

    pub fn mouse_up(&mut self, x: f32, y: f32, mouse_button: MouseButton, time_stamp: f32) {
        self.user_input
            .mouse_up(mouse_button, Vector2::new(x, y), time_stamp);
    }

    pub fn mouse_move(&mut self, x: f32, y: f32, time_stamp: f32) {
        self.user_input.mouse_move(Vector2::new(x, y), time_stamp);
    }
}
