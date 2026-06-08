use chubby_bunny::{Body, BodyId, DistanceConstraint, Particle};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct BodyMeta {
    pub id: BodyId,
    pub z_index: i32,
    pub line_color: Color,
    pub fill_color: Color,
}

#[wasm_bindgen]
struct PlaygroundState {}

#[wasm_bindgen]
pub struct Playground {
    state: PlaygroundState,
    bodies: HashMap<BodyId, Body>,
}

#[wasm_bindgen]
impl Playground {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Playground {
        Playground {
            state: PlaygroundState {},
            bodies: HashMap::new(),
        }
    }

    pub fn init(&mut self) {
        // create scene here
    }

    pub fn update(&mut self, dt: f32) {
        // advance simulation by dt here
    }

    pub fn point_count(&self) -> usize {
        0
    }

    pub fn point_x(&self, _index: usize) -> f32 {
        0.0
    }

    pub fn point_y(&self, _index: usize) -> f32 {
        0.0
    }

    pub fn line_count(&self) -> usize {
        0
    }

    pub fn line_a(&self, _index: usize) -> usize {
        0
    }

    pub fn line_b(&self, _index: usize) -> usize {
        0
    }
}
