use chubby_bunny::{
    AttachmentConstraint, Body, BodyId, CollisionConstraint, ExtrinsicConstraintType, Particle,
    SolverSettings, WallConstraint,
};
use nalgebra::Vector2;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

mod primitives;
use primitives::{create_polygon, create_quad};

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
fn create_container(width: usize, height: usize) -> Body {
    let mut container_body = Body::empty();
    let mut create_particle_helper = |x, y| {
        container_body.particles.push(Particle::new(
            nalgebra::Vector2::new(x as f32, y as f32),
            nalgebra::Vector2::new(0.0, 0.0),
            1.0,
            0.01,
            true,
        ));
    };
    create_particle_helper(0, height);
    create_particle_helper(width, height);
    create_particle_helper(width, 0);
    create_particle_helper(0, 0);

    for i in 0..4 {
        container_body
            .children_constraints
            .push(ExtrinsicConstraintType::Global(Box::new(WallConstraint {
                parent_point_idx_end: i,
                parent_point_idx_origin: (i + 1) % 4,
                stiffness: 1.0,
            })));
    }
    container_body
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

    pub fn init(&mut self, width: usize, height: usize) {
        let mut simple_quad = create_quad(Vector2::new(0.0, 100.0), 100.0, 0.1, 0.3, 0.0, 0.01);
        let third_quad = create_quad(Vector2::new(200.0, 200.0), 50.0, 0.5, 0.3, 0.0, 0.01);
        let fourth_quad =
            create_polygon(Vector2::new(300.0, 100.0), 75.0, 12, 0.6, 0.95, 0.0, 0.01);
        let ball = create_polygon(Vector2::new(300.0, 200.0), 80.0, 20, 0.95, 0.6, 0.0, 0.01);
        let small_quad = create_quad(Vector2::new(25.0, 125.0), 25.0, 0.5, 0.3, 0.0, 0.01);

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
        let mut container_body = create_container(width, height);

        container_body.children.push(simple_quad);
        container_body.children.push(third_quad);
        container_body.children.push(fourth_quad);
        container_body.children.push(ball);
        container_body.collision_constraint = Some(CollisionConstraint::new(0.2));
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
        let reference_dt = 1.0 / 60.0;
        let dt = (dt_ms / 1000.0).min(2.0 * reference_dt); //someone left the tab etc...
        for body in self.bodies.iter_mut() {
            let constant_force =
                chubby_bunny::force::constant_force(nalgebra::Vector2::new(0.0, 400.0));
            let _constant_force2 =
                chubby_bunny::force::constant_force(nalgebra::Vector2::new(30.0, 0.0));
            let settings = SolverSettings {
                reference_dt,
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
