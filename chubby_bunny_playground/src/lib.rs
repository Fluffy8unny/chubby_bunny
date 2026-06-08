use chubby_bunny::{AreaConstraint, Body, BodyId, DistanceConstraint, Particle, WallConstraint};
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
    target_fps: f32,
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
            target_fps: 60.0,
        }
    }

    pub fn init(&mut self) {
        let mut simple_quad = Body::empty();

        simple_quad.particles.push(Particle {
            position: nalgebra::Vector2::new(0.0, 0.0),
            velocity: nalgebra::Vector2::new(0.0, 0.0),
            mass: 1.0,
            friction: 0.001,
            pinned: false,
        });
        simple_quad.particles.push(Particle {
            position: nalgebra::Vector2::new(100.0, 0.0),
            velocity: nalgebra::Vector2::new(0.0, 0.0),
            mass: 1.0,
            friction: 0.001,
            pinned: false,
        });
        simple_quad.particles.push(Particle {
            position: nalgebra::Vector2::new(100.0, 100.0),
            velocity: nalgebra::Vector2::new(0.0, 0.0),
            mass: 1.0,
            friction: 0.001,
            pinned: false,
        });
        simple_quad.particles.push(Particle {
            position: nalgebra::Vector2::new(0.0, 100.0),
            velocity: nalgebra::Vector2::new(0.0, 0.0),
            mass: 1.0,
            friction: 0.001,
            pinned: false,
        });

        let stiffness = 0.5;
        simple_quad
            .constraints
            .push(Box::new(DistanceConstraint::new(
                0,
                1,
                &simple_quad.particles,
                stiffness,
                self.target_fps,
            )));
        simple_quad
            .constraints
            .push(Box::new(DistanceConstraint::new(
                1,
                2,
                &simple_quad.particles,
                stiffness,
                self.target_fps,
            )));
        simple_quad
            .constraints
            .push(Box::new(DistanceConstraint::new(
                2,
                3,
                &simple_quad.particles,
                stiffness,
                self.target_fps,
            )));
        simple_quad
            .constraints
            .push(Box::new(DistanceConstraint::new(
                3,
                0,
                &simple_quad.particles,
                stiffness,
                self.target_fps,
            )));
        simple_quad.constraints.push(Box::new(AreaConstraint::new(
            vec![0, 1, 2, 3],
            &simple_quad.particles,
            0.5,
            self.target_fps,
        )));

        let mut container_body = Body::empty();
        container_body.particles.push(Particle {
            position: nalgebra::Vector2::new(0.0, 500.0),
            velocity: nalgebra::Vector2::new(0.0, 0.0),
            mass: 1.0,
            friction: 0.01,
            pinned: true,
        });
        container_body.particles.push(Particle {
            position: nalgebra::Vector2::new(500.0, 500.0),
            velocity: nalgebra::Vector2::new(0.0, 0.0),
            mass: 1.0,
            friction: 0.01,
            pinned: true,
        });
        container_body.particles.push(Particle {
            position: nalgebra::Vector2::new(500.0, 0.0),
            velocity: nalgebra::Vector2::new(0.0, 0.0),
            mass: 1.0,
            friction: 0.01,
            pinned: true,
        });
        container_body.particles.push(Particle {
            position: nalgebra::Vector2::new(0.0, 0.0),
            velocity: nalgebra::Vector2::new(0.0, 0.0),
            mass: 1.0,
            friction: 0.01,
            pinned: true,
        });
        let quad_id = simple_quad.id;
        container_body.children.push(simple_quad);
        container_body
            .children_constraints
            .push(Box::new(WallConstraint {
                idx_body: quad_id,
                parent_point_idx_origin: 1,
                parent_point_idx_end: 0,
                stiffness: 1.0,
                fps: self.target_fps,
            }));
        container_body
            .children_constraints
            .push(Box::new(WallConstraint {
                idx_body: quad_id,
                parent_point_idx_origin: 2,
                parent_point_idx_end: 1,
                stiffness: 1.0,
                fps: self.target_fps,
            }));
        self.bodies.insert(container_body.id, container_body);
    }

    pub fn update(&mut self, dt_ms: f32) {
        let dt = dt_ms / 1000.0;
        for body in self.bodies.values_mut() {
            let constant_force =
                chubby_bunny::force::constant_force(nalgebra::Vector2::new(0.0, 20.0));
            let constant_force2 =
                chubby_bunny::force::constant_force(nalgebra::Vector2::new(10.0, 0.0));
            body.perform_step(&vec![constant_force, constant_force2], dt);
        }
        self.polygon_arrays = bodies_to_polygon_arrays(self.bodies.values(), &self.meta_data);
    }

    pub fn get_polygon_arrays(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.polygon_arrays)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
