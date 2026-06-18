use chubby_bunny_core::{
    Body, BodyId, CollisionConstraint, ExtrinsicConstraintType, Particle, SolverSettings,
    Transformation, WallConstraint,
};
use chubby_bunny_svg::{load_svg, BodyMeta, BodySettings};

use nalgebra::Vector2;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

mod primitives;
pub use primitives::{create_polygon, create_quad, create_rect}; //keeping this pub to surpress warnings atm

pub mod js_types;
use js_types::{
    bodies_to_polygon_arrays, default_meta_for_container, EventType, OutgoingEvent, PolygonArray,
};

mod input;
use input::{InputState, MouseButton};

mod spawner;
use spawner::BunnySpawner;

fn create_container(width: usize, height: usize, max_scale: f32) -> Body {
    let mut container_body = Body::empty();
    let mut create_particle_helper = |x, y| {
        container_body.particles.push(Particle::new(
            nalgebra::Vector2::new(x, y),
            nalgebra::Vector2::new(0.0, 0.0),
            1.0,
            0.01,
            true,
        ));
    };
    create_particle_helper(0_f32, height as f32);
    create_particle_helper(width as f32, height as f32);
    create_particle_helper(width as f32, -max_scale);
    create_particle_helper(0_f32, -max_scale);

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
    spawner: BunnySpawner<f32>,
    interactive_bodies: HashMap<BodyId, String>,
    gravity: Vector2<f32>,
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
            spawner: BunnySpawner::new(1000.0, 50, 1200.0, 150.0, 250.0),
            interactive_bodies: HashMap::new(),
            gravity: Vector2::new(0.0, 250.0),
        }
    }

    fn load_svg_file(
        &mut self,
        svg_data: &str,
        svg_instance_transform: Transformation<f32>,
        settings: &BodySettings<f32>,
    ) -> Vec<Body> {
        let (mut template, meta) = load_svg(svg_data, settings);
        template.iter_mut().for_each(|template| template.transform(svg_instance_transform));
        self.meta_data.extend(meta);
        template
    }

    pub fn init(&mut self, width: usize, height: usize) {
        self.bodies.clear();
        self.polygon_arrays.clear();
        self.meta_data.clear();
        self.current_selected_body.clear();
        self.interactive_bodies.clear();
        self.user_input = InputState::new();
        self.spawner.reset_runtime_state();

        self.spawner.update_settings(width, height);
        let mut container_body = create_container(width, height, self.spawner.get_scale());
        let svg_settings =
            BodySettings::from_values(1.0, 0.01, false, 0.5, 0.35, 0.3, 0.4, 0.5, 5, 8, 2.0, 3);

        let button_scale = width.min(height) as f32 / 6.0;
        let button_drop_off = height as f32 - button_scale * 2.0;
        let get_transform = |offset_x| Transformation {
            offset: Vector2::new(offset_x, button_drop_off),
            scale: button_scale,
            rotation_radians: 0.0,
        };

        for (svg_data, transform, name) in [
            (
                include_str!("../../assets/mail.svg"),
                get_transform(width as f32 / 2.0 - button_scale * 1.5),
                "mail",
            ),
            (
                include_str!("../../assets/git.svg"),
                get_transform(width as f32 / 2.0 - button_scale * 0.5),
                "git",
            ),
            (
                include_str!("../../assets/about.svg"),
                get_transform(width as f32 / 2.0 + button_scale * 0.5),
                "about",
            ),
        ] {
            let svg_instance = self.load_svg_file(svg_data, transform, &svg_settings);
            for body in &svg_instance {
                self.interactive_bodies.insert(body.id, name.to_string());
            }
            container_body.children.extend(svg_instance);
        }
        let cloud_settings =
            BodySettings::from_values(1.0, 0.01, false, 1.0, 1.0, 0.6, 0.8, 0.5, 5, 8, 2.0, 3);

        let mut svg_instance = self.load_svg_file(
            include_str!("../../assets/clouds_foreground.svg"),
            Transformation {
                offset: Vector2::new(0.0, height as f32 - width as f32 / 16.0),
                scale: width as f32,
                rotation_radians: 0.0,
            },
            &cloud_settings,
        );

        container_body.children.append(&mut svg_instance);
        self.spawner.load_bunnies_from_svg(
            vec![
                include_str!("../../assets/t1.svg"),
                include_str!("../../assets/t2.svg"),
                include_str!("../../assets/t3.svg"),
                include_str!("../../assets/t4.svg"),
            ],
            svg_settings,
        );
        self.meta_data.insert(
            container_body.id,
            default_meta_for_container(container_body.id),
        );

        container_body.collision_constraint = Some(CollisionConstraint::new(0.99));
        self.bodies.push(container_body);
        self.gravity = Vector2::new(0.0, height as f32 / 10.0);
    }

    pub fn reset(&mut self, width: f32, height: f32) {
        self.init(width as usize, height as usize);
    }

    fn handle_selection(&mut self, position: Vector2<f32>, time_stamp: f32) -> Vec<OutgoingEvent> {
        let mut interactive_body_selected = Vec::new();
        for container in self.bodies.iter_mut() {
            for body in container.children.iter_mut() {
                if body.point_in_polygon(position) {
                    self.current_selected_body.push(body.id);
                    body.pin_child_by_id(body.id, true);
                    if let Some(name) = self.interactive_bodies.get(&body.id) {
                        interactive_body_selected.push(OutgoingEvent {
                            event_type: EventType::Selection,
                            body_id: body.id,
                            description: name.clone(),
                            time_stamp,
                        });
                    }
                }
            }
        }
        interactive_body_selected
    }

    fn handle_deselection(&mut self, time_stamp: f32) -> Vec<OutgoingEvent> {
        let mut outgoing_events = Vec::new();
        while let Some(selected_body) = self.current_selected_body.pop() {
            for container in self.bodies.iter_mut() {
                container.pin_child_by_id(selected_body, false);
            }
            //todo add velocity to the body based on the average velocity of the mouse during the drag
            if let Some(name) = self.interactive_bodies.get(&selected_body) {
                outgoing_events.push(OutgoingEvent {
                    event_type: EventType::Deselection,
                    body_id: selected_body,
                    description: name.clone(),
                    time_stamp,
                });
            }
        }
        outgoing_events
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
                    container.set_movement_of_child_by_id(*selected_body, avg_displacement);
                }
            }
        } else {
            web_sys::console::log_1(
                &"Not enough data for average displacement and time delta.".into(),
            );
        }
    }

    pub fn update(&mut self, dt_ms: f32) -> Result<JsValue, JsValue> {
        //handle user input
        let mut outgoing_events = Vec::new();
        while let Some(event) = self.user_input.events.pop_front() {
            match event.event_type {
                input::MouseEventType::Down => {
                    if event.button == MouseButton::Left {
                        outgoing_events.extend(
                            self.handle_selection(
                                event.state.mouse_position,
                                event.state.time_stamp,
                            ),
                        );
                    }
                }
                input::MouseEventType::Up => {
                    if event.button == MouseButton::Left {
                        outgoing_events.extend(self.handle_deselection(event.state.time_stamp));
                    }
                }
                input::MouseEventType::Move => {
                    self.handle_drag(event.button);
                }
            }
        }

        if let Some((body, meta)) = self.spawner.update(dt_ms) {
            self.meta_data.extend(meta);
            self.bodies[0].children.push(body);
        };

        let dt: f32 = dt_ms / 1000.0;
        for body in self.bodies.iter_mut() {
            let constant_force = chubby_bunny_core::force::constant_force(self.gravity);
            let settings = SolverSettings {
                reference_dt: 1.0 / 60.0,
                constraint_iterations: 8,
            };
            let capped_dt = dt.min(2.0 * settings.reference_dt);
            body.perform_step(&vec![constant_force], capped_dt, &settings);
        }

        self.polygon_arrays = bodies_to_polygon_arrays(
            self.bodies.iter(),
            &self.meta_data,
            &self.current_selected_body,
        );
        serde_wasm_bindgen::to_value(&outgoing_events)
            .map_err(|e| JsValue::from_str(&e.to_string()))
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
