use chubby_bunny_core::{
    AttachmentConstraint, Body, BodyId, CollisionConstraint, ExtrinsicConstraintType,
    FloatingPointNumber, Particle, SolverSettings, Transformation, WallConstraint,
};
use chubby_bunny_svg::{
    instantiate_svg_bodies, instantiate_svg_body, load_svg, BodyMeta, BodySettings,
};

use nalgebra::{zero, Vector2};
use rand::prelude::IndexedRandom;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

mod primitives;
use primitives::{create_polygon, create_quad};

pub mod js_types;
use js_types::{bodies_to_polygon_arrays, default_meta_for_container, PolygonArray};

mod input;
use input::{InputState, MouseButton};

struct BunnySpawner<T = f32> {
    number_of_bunnies: usize,
    max_bunnies: usize,
    spawn_timer: f32,
    spawn_interval: f32,
    bunny_bodies: Vec<Body<T>>,
    bunny_meta: Vec<HashMap<BodyId, BodyMeta>>,
    min_pos_x: T,
    max_pos_x: T,
    y_pos: T,
    min_scale: T,
    max_scale: T,
    svg_settings: Option<BodySettings<T>>,
}

impl<T: FloatingPointNumber> BunnySpawner<T> {
    pub fn new(
        spawn_interval: f32,
        max_bunnies: usize,
        max_pos: T,
        y_pos: T,
        min_scale: T,
        max_scale: T,
    ) -> Self {
        Self {
            number_of_bunnies: 0,
            max_bunnies,
            spawn_timer: 0.0,
            spawn_interval,
            bunny_bodies: Vec::new(),
            bunny_meta: Vec::new(),
            min_pos_x: T::zero(),
            max_pos_x: max_pos,
            y_pos,
            min_scale,
            max_scale,
            svg_settings: None,
        }
    }
    fn spawn_bunny(&mut self) -> Option<(Body<T>, HashMap<BodyId, BodyMeta>)> {
        let xpos =
            T::from(rand::random::<f32>()) * (self.max_pos_x - self.min_pos_x) + self.min_pos_x;
        let scale =
            T::from(rand::random::<f32>()) * (self.max_scale - self.min_scale) + self.min_scale;
        let svg_instance_transform = Transformation {
            offset: Vector2::new(xpos, self.y_pos),
            scale: scale,
            rotation_radians: T::zero(),
        };
        let picked_bunny: usize = rand::random_range(0..self.bunny_bodies.len());
        Some(instantiate_svg_body(
            self.bunny_bodies.get(picked_bunny)?,
            self.bunny_meta.get(picked_bunny)?,
            svg_instance_transform,
        ))
    }

    pub fn spawn_bubbles(
        &mut self,
        center_position: Vector2<T>,
        count: usize,
        scale: T,
    ) -> Option<Vec<(Body<T>, HashMap<BodyId, BodyMeta>)>> {
        (0..count)
            .map(|_i| {
                let random_offset = Vector2::new(T::zero(), T::zero()) - center_position;
                let ball = create_polygon(
                    center_position + random_offset,
                    scale,
                    20,
                    T::from_f32(0.95).unwrap(),
                    T::from_f32(0.6).unwrap(),
                    T::from_f32(0.0).unwrap(),
                    T::from_f32(0.01).unwrap(),
                );

                let meta = self.bunny_meta.choose(&mut rand::rng())?.clone();
                Some((ball, meta))
            })
            .collect::<Option<Vec<_>>>()
    }

    pub fn update(&mut self, dt: f32) -> Option<(Body<T>, HashMap<BodyId, BodyMeta>)> {
        if self.number_of_bunnies >= self.max_bunnies {
            return None;
        }
        self.spawn_timer += dt;
        if self.spawn_timer >= self.spawn_interval {
            self.spawn_timer -= self.spawn_interval;
            self.number_of_bunnies += 1;
            Some(self.spawn_bunny()?)
        } else {
            None
        }
    }
    pub fn update_settings(&mut self, width: usize, height: usize) {
        self.max_scale = T::from_usize(width.min(height) / 5).unwrap();
        self.min_scale = T::from_usize(width.min(height) / 20).unwrap();
        self.min_pos_x = T::zero();
        self.max_pos_x = T::from_usize(width).unwrap() - self.max_scale;
    }
    pub fn load_bunnies_from_svg(&mut self, svg_data: Vec<&str>, settings: BodySettings<T>) {
        self.svg_settings = Some(settings);
        for svg_path in svg_data.iter() {
            let (mut bodies, meta) = load_svg(svg_path, self.svg_settings.as_ref().unwrap());
            self.bunny_bodies.append(&mut bodies);
            self.bunny_meta.push(meta);
        }
    }
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
pub struct Playground {
    bodies: Vec<Body>,
    polygon_arrays: Vec<PolygonArray>,
    meta_data: HashMap<BodyId, BodyMeta>,
    user_input: InputState,
    current_selected_body: Vec<BodyId>,
    spawner: BunnySpawner<f32>,
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
            spawner: BunnySpawner::new(1000.0, 50, 1200.0, 50.0, 150.0, 350.0),
        }
    }
    pub fn init(&mut self, width: usize, height: usize) {
        let mut container_body = create_container(width, height);
        let mut svg_settings = BodySettings::from_values(1.0, 0.01, false, 0.5, 0.2, 0.5, 0.2, 0.5);
        svg_settings.attachment_settings.child_sample_stride = 5;
        svg_settings.attachment_settings.max_total_attachments = 8;
        svg_settings
            .attachment_settings
            .parent_springs_per_child_anchor = 3;

        self.spawner.load_bunnies_from_svg(
            vec![
                include_str!("../../assets/t1.svg"),
                include_str!("../../assets/t2.svg"),
                include_str!("../../assets/t3.svg"),
                include_str!("../../assets/t4.svg"),
            ],
            svg_settings,
        );
        self.spawner.update_settings(width, height);
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
        self.spawner.update(dt_ms).map(|(body, meta)| {
            self.meta_data.extend(meta);
            self.bodies[0].children.push(body);
        });
        let dt = dt_ms / 1000.0;
        for body in self.bodies.iter_mut() {
            let constant_force =
                chubby_bunny_core::force::constant_force(nalgebra::Vector2::new(0.0, 300.0));
            let settings = SolverSettings {
                reference_dt: 1.0 / 60.0,
                constraint_iterations: 5,
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
