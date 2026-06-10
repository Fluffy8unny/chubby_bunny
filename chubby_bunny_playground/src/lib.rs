use chubby_bunny::{
    AreaConstraint, AttachmentConstraint, Body, BodyId, CollisionConstraint, DistanceConstraint,
    ExtrinsicConstraintType, Particle, SolverSettings, WallConstraint,
};
use nalgebra::Vector2;
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
    bodies: Vec<Body>,
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

fn create_quad(
    start: Vector2<f32>,
    size: f32,
    stiffness_distance: f32,
    stiffness_shear: f32,
    stiffness_area: f32,
) -> Body {
    let mut body = Body::empty();
    body.particles.push(Particle::new(
        start,
        nalgebra::Vector2::new(0.0, 0.0),
        1.0,
        0.001,
        false,
    ));
    body.particles.push(Particle::new(
        start + Vector2::new(size, 0.0),
        nalgebra::Vector2::new(0.0, 0.0),
        1.0,
        0.001,
        false,
    ));
    body.particles.push(Particle::new(
        start + Vector2::new(size, size),
        nalgebra::Vector2::new(0.0, 0.0),
        1.0,
        0.001,
        false,
    ));
    body.particles.push(Particle::new(
        start + Vector2::new(0.0, size),
        nalgebra::Vector2::new(0.0, 0.0),
        1.0,
        0.001,
        false,
    ));

    for i in 0..4 {
        body.constraints.push(Box::new(DistanceConstraint::new(
            i,
            (i + 1) % 4,
            &body.particles,
            stiffness_distance,
        )));
    }
    for i in 0..2 {
        body.constraints.push(Box::new(DistanceConstraint::new(
            i,
            (i + 2) % 4,
            &body.particles,
            stiffness_shear,
        )));
    }

    body.constraints.push(Box::new(AreaConstraint::new(
        vec![0, 1, 2, 3],
        &body.particles,
        stiffness_area,
    )));
    body
}
#[wasm_bindgen]
impl Playground {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Playground {
        Playground {
            bodies: Vec::new(),
            polygon_arrays: Vec::new(),
            meta_data: HashMap::new(),
        }
    }

    pub fn init(&mut self) {
        let mut simple_quad = create_quad(Vector2::new(0.0, 0.0), 100.0, 0.1, 0.3, 0.1);
        let third_quad = create_quad(Vector2::new(200.0, 0.0), 50.0, 0.5, 0.3, 0.5);
        let fourth_quad = create_quad(Vector2::new(300.0, 0.0), 75.0, 0.3, 0.3, 0.5);
        let fifth = create_quad(Vector2::new(300.0, 200.0), 50.0, 0.3, 0.3, 0.5);
        let small_quad = create_quad(Vector2::new(25.0, 25.0), 25.0, 0.5, 0.3, 0.5);

        simple_quad
            .children_constraints
            .push(ExtrinsicConstraintType::Local(Box::new(
                AttachmentConstraint::new(
                    small_quad.id,
                    &simple_quad,
                    &small_quad,
                    vec![0, 1, 2, 3],
                    vec![0, 1, 2, 3],
                    0.5,
                ),
            )));

        simple_quad.children.push(small_quad);
        simple_quad
            .children_constraints
            .push(ExtrinsicConstraintType::Global(Box::new(WallConstraint {
                parent_point_idx_origin: 0,
                parent_point_idx_end: 1,
                stiffness: 0.95,
            })));
        simple_quad
            .children_constraints
            .push(ExtrinsicConstraintType::Global(Box::new(WallConstraint {
                parent_point_idx_origin: 1,
                parent_point_idx_end: 2,
                stiffness: 0.95,
            })));
        simple_quad
            .children_constraints
            .push(ExtrinsicConstraintType::Global(Box::new(WallConstraint {
                parent_point_idx_origin: 2,
                parent_point_idx_end: 3,
                stiffness: 0.95,
            })));
        simple_quad
            .children_constraints
            .push(ExtrinsicConstraintType::Global(Box::new(WallConstraint {
                parent_point_idx_origin: 3,
                parent_point_idx_end: 0,
                stiffness: 0.95,
            })));

        let mut container_body = Body::empty();
        container_body.particles.push(Particle::new(
            nalgebra::Vector2::new(0.0, 100.0),
            nalgebra::Vector2::new(0.0, 0.0),
            1.0,
            0.01,
            true,
        ));
        container_body.particles.push(Particle::new(
            nalgebra::Vector2::new(500.0, 500.0),
            nalgebra::Vector2::new(0.0, 0.0),
            1.0,
            0.01,
            true,
        ));
        container_body.particles.push(Particle::new(
            nalgebra::Vector2::new(500.0, 0.0),
            nalgebra::Vector2::new(0.0, 0.0),
            1.0,
            0.01,
            true,
        ));
        container_body.particles.push(Particle::new(
            nalgebra::Vector2::new(0.0, 0.0),
            nalgebra::Vector2::new(0.0, 0.0),
            1.0,
            0.01,
            true,
        ));
        container_body.children.push(simple_quad);
        container_body.children.push(third_quad);
        container_body.children.push(fourth_quad);
        container_body.children.push(fifth);
        container_body.collision_constraint = Some(CollisionConstraint::new(0.8));
        container_body
            .children_constraints
            .push(ExtrinsicConstraintType::Global(Box::new(WallConstraint {
                parent_point_idx_origin: 1,
                parent_point_idx_end: 0,
                stiffness: 1.0,
            })));
        container_body
            .children_constraints
            .push(ExtrinsicConstraintType::Global(Box::new(WallConstraint {
                parent_point_idx_origin: 2,
                parent_point_idx_end: 1,
                stiffness: 1.0,
            })));
        self.bodies.push(container_body);
    }

    pub fn update(&mut self, dt_ms: f32) {
        let dt = dt_ms / 1000.0;
        for body in self.bodies.iter_mut() {
            let constant_force =
                chubby_bunny::force::constant_force(nalgebra::Vector2::new(0.0, 300.0));
            let _constant_force2 =
                chubby_bunny::force::constant_force(nalgebra::Vector2::new(30.0, 0.0));
            let settings = SolverSettings {
                reference_dt: 1.0 / 60.0,
                constraint_iterations: 5,
            };
            body.perform_step(&vec![constant_force], dt, &settings);
        }
        self.polygon_arrays = bodies_to_polygon_arrays(self.bodies.iter(), &self.meta_data);
    }

    pub fn get_polygon_arrays(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.polygon_arrays)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

impl Default for Playground {
    fn default() -> Self {
        Self::new()
    }
}
