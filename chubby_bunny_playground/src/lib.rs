use chubby_bunny::{Body, BodyId, DistanceConstraint, Particle};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[wasm_bindgen]
#[derive(Debug, Clone, serde::Serialize)]
pub struct BodyMeta {
    pub id: BodyId,
    pub z_index: i32,
    pub line_color: Color,
    pub fill_color: Color,
}

#[derive(serde::Serialize)]
struct PolygonArray {
    vertices: Vec<(f32, f32)>,
    edges: Vec<(u32, u32)>,
    meta: BodyMeta,
    z_index: i32,
    children: Vec<PolygonArray>,
}

#[wasm_bindgen]
pub struct Playground {
    bodies: HashMap<BodyId, Body>,
    polygon_arrays: Vec<PolygonArray>,
    meta_data: HashMap<BodyId, BodyMeta>,
}

fn default_meta(id: BodyId, z_index: i32) -> BodyMeta {
    BodyMeta {
        id,
        z_index,
        line_color: Color {
            r: 30,
            g: 30,
            b: 30,
        },
        fill_color: Color {
            r: 190,
            g: 190,
            b: 190,
        },
    }
}

fn body_to_polygon_array(
    body: &Body,
    meta_data: &HashMap<BodyId, BodyMeta>,
    depth: i32,
) -> PolygonArray {
    let vertices: Vec<(f32, f32)> = body
        .particles
        .iter()
        .map(|p| (p.position.x, p.position.y))
        .collect();
    let n = vertices.len();
    let edges: Vec<(u32, u32)> = (0..n).map(|i| (i as u32, ((i + 1) % n) as u32)).collect();
    let children = body
        .children
        .iter()
        .map(|child| body_to_polygon_array(child, meta_data, depth + 1))
        .collect();
    let meta = meta_data
        .get(&body.id)
        .cloned()
        .unwrap_or_else(|| default_meta(body.id, depth));

    PolygonArray {
        vertices,
        edges,
        meta,
        z_index: depth,
        children,
    }
}

fn bodies_to_polygon_arrays<'a, I>(
    bodies: I,
    meta_data: &HashMap<BodyId, BodyMeta>,
) -> Vec<PolygonArray>
where
    I: IntoIterator<Item = &'a Body>,
{
    bodies
        .into_iter()
        .map(|body| body_to_polygon_array(body, meta_data, 0))
        .collect()
}

#[wasm_bindgen]
impl Playground {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Playground {
        Playground {
            bodies: HashMap::new(),
            polygon_arrays: Vec::new(),
            meta_data: HashMap::new(),
        }
    }

    pub fn init(&mut self) {
        let mut simple_quad = Body::empty();
        simple_quad.particles.push(Particle {
            position: nalgebra::Vector2::new(0.0, 0.0),
            velocity: nalgebra::Vector2::new(0.0, 0.0),
            mass: 1.0,
            friction: 0.01,
            pinned: false,
        });
        simple_quad.particles.push(Particle {
            position: nalgebra::Vector2::new(100.0, 0.0),
            velocity: nalgebra::Vector2::new(0.0, 0.0),
            mass: 1.0,
            friction: 0.01,
            pinned: false,
        });
        simple_quad.particles.push(Particle {
            position: nalgebra::Vector2::new(100.0, 100.0),
            velocity: nalgebra::Vector2::new(0.0, 0.0),
            mass: 1.0,
            friction: 0.01,
            pinned: false,
        });
        simple_quad.particles.push(Particle {
            position: nalgebra::Vector2::new(0.0, 100.0),
            velocity: nalgebra::Vector2::new(0.0, 0.0),
            mass: 1.0,
            friction: 0.01,
            pinned: false,
        });
        simple_quad
            .constraints
            .push(Box::new(DistanceConstraint::new(
                0,
                1,
                &simple_quad.particles,
                1.0,
            )));
        simple_quad
            .constraints
            .push(Box::new(DistanceConstraint::new(
                1,
                2,
                &simple_quad.particles,
                1.0,
            )));
        simple_quad
            .constraints
            .push(Box::new(DistanceConstraint::new(
                2,
                3,
                &simple_quad.particles,
                1.0,
            )));
        simple_quad
            .constraints
            .push(Box::new(DistanceConstraint::new(
                3,
                0,
                &simple_quad.particles,
                1.0,
            )));

        self.bodies.insert(simple_quad.id, simple_quad);
    }

    pub fn update(&mut self, dt: f32) {
        for body in self.bodies.values_mut() {
            let constant_force =
                chubby_bunny::force::constant_force(nalgebra::Vector2::new(0.0, 0.001));
            let constant_force2 =
                chubby_bunny::force::constant_force(nalgebra::Vector2::new(0.001, 0.0));
            body.perform_step(&vec![constant_force, constant_force2], dt);
        }
        self.polygon_arrays = bodies_to_polygon_arrays(self.bodies.values(), &self.meta_data);
    }

    pub fn get_polygon_arrays(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.polygon_arrays)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
