use chubby_bunny::{
    AttachmentConstraint, Body, BodyId, CollisionConstraint, ExtrinsicConstraintType, Particle,
    SolverSettings, WallConstraint,
};

use nalgebra::Vector2;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

mod primitives;
use primitives::{create_polygon, create_quad};

pub mod js_types;
use js_types::{bodies_to_polygon_arrays, default_meta_for_container, BodyMeta, PolygonArray};

mod input;
use input::{InputState, MouseButton};

mod svg;
use svg::{load_svg, BodySettings};

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
pub struct Playground {
    bodies: Vec<Body>,
    polygon_arrays: Vec<PolygonArray>,
    meta_data: HashMap<BodyId, BodyMeta>,
    user_input: InputState,
    current_selected_body: Vec<BodyId>,
}
#[wasm_bindgen]
impl Playground {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Playground {
        Playground {
            bodies: Vec::new(),
            polygon_arrays: Vec::new(),
            meta_data: HashMap::new(),
            user_input: InputState::new(),
            current_selected_body: Vec::new(),
        }
    }
    pub fn init(&mut self, width: usize, height: usize) {
        let mut simple_quad = create_quad(Vector2::new(0.0, 100.0), 100.0, 0.9, 0.3, 0.0, 0.01);
        let third_quad = create_quad(Vector2::new(200.0, 200.0), 50.0, 0.9, 0.3, 0.0, 0.01);
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
        let mut svg_settings =
            BodySettings::from_values(1.0, 0.01, false, 0.9, 0.9, 0.5, 0.01, 0.5);
        svg_settings.attachment_settings.child_sample_stride = 5;
        svg_settings.attachment_settings.max_total_attachments = 8;
        svg_settings
            .attachment_settings
            .parent_springs_per_child_anchor = 3;
        let (mut test_body, test_meta_data) =
            load_svg(include_str!("../../assets/t1.svg"), &svg_settings);
        for body in test_body.iter_mut() {
            body.move_uniform(Vector2::new(300.0, 300.0), Vector2::new(300.0, 300.0));
        }
        for body in test_body {
            container_body.children.push(body);
        }
        self.meta_data.extend(test_meta_data);
        self.meta_data.insert(
            container_body.id,
            default_meta_for_container(container_body.id),
        );
        container_body.collision_constraint = Some(CollisionConstraint::new(0.99));
        self.bodies.push(container_body);
    }

    fn handle_selection(&mut self, position: Vector2<f32>) {
        for container in self.bodies.iter_mut() {
            for body in container.children.iter_mut() {
                if body.point_in_polygon(position) {
                    self.current_selected_body.push(body.id);
                    body.pin_child_by_id(body.id, true);
                }
            }
        }
    }

    fn handle_deselection(&mut self) {
        while let Some(selected_body) = self.current_selected_body.pop() {
            for container in self.bodies.iter_mut() {
                container.pin_child_by_id(selected_body, false);
            }
            //todo add velocity to the body based on the average velocity of the mouse during the drag
        }
    }

    fn handle_drag(&mut self, button: MouseButton) {
        if button != MouseButton::Left {
            return;
        }
        if let Some((avg_displacement, _avg_time_delta)) = self
            .user_input
            .get_average_mouse_displacement_and_time_delta(button, 5)
        {
            for container in self.bodies.iter_mut() {
                for selected_body in &self.current_selected_body {
                    container.move_child_by_id(*selected_body, avg_displacement);
                }
            }
        } else {
            web_sys::console::log_1(
                &"Not enough data for average displacement and time delta.".into(),
            );
        }
    }

    pub fn update(&mut self, dt_ms: f32) {
        //handle user input
        while let Some(event) = self.user_input.events.pop_front() {
            match event.event_type {
                input::MouseEventType::Down => {
                    if event.button == MouseButton::Left {
                        self.handle_selection(event.state.mouse_position);
                    }
                }
                input::MouseEventType::Up => {
                    if event.button == MouseButton::Left {
                        self.handle_deselection();
                    }
                }
                input::MouseEventType::Move => {
                    self.handle_drag(event.button);
                }
            }
        }

        let dt = dt_ms / 1000.0;
        for body in self.bodies.iter_mut() {
            let constant_force =
                chubby_bunny::force::constant_force(nalgebra::Vector2::new(0.0, 400.0));
            let settings = SolverSettings {
                reference_dt: 1.0 / 60.0,
                constraint_iterations: 20,
            };
            body.perform_step(&vec![constant_force], dt, &settings);
        }
        self.polygon_arrays = bodies_to_polygon_arrays(
            self.bodies.iter(),
            &self.meta_data,
            &self.current_selected_body,
        );
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

impl Default for Playground {
    fn default() -> Self {
        Self::new()
    }
}
