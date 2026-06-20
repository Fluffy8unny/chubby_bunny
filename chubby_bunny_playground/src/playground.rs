use crate::game_loop::{Game, GameLoop};
use crate::input::{Event, MouseButton, MouseEventType};
use crate::js_types::{default_meta_for_container, EventType, OutgoingEvent};
use crate::spawner::BunnySpawner;
use chubby_bunny_bindgen::chubby_bunny_bindgen;
use chubby_bunny_core::{
    Body, BodyId, CollisionConstraint, ExtrinsicConstraintType, Particle, SolverSettings,
    Transformation, WallConstraint,
};
use chubby_bunny_svg::{load_svg, BodyMeta, BodySettings};

use nalgebra::Vector2;
use std::collections::{HashMap, VecDeque};

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

struct PlaygroundGame {
    bodies: Vec<Body>,
    meta_data: HashMap<BodyId, BodyMeta>,
    current_selection: Vec<BodyId>,
    spawner: BunnySpawner<f32>,
    interactive_bodies: HashMap<BodyId, String>,
    gravity: Vector2<f32>,
}
impl PlaygroundGame {
    pub fn new() -> PlaygroundGame {
        PlaygroundGame {
            bodies: Vec::new(),
            meta_data: HashMap::new(),
            current_selection: Vec::new(),
            spawner: BunnySpawner::new(1000.0, 50, 1200.0, 150.0, 250.0),
            interactive_bodies: HashMap::new(),
            gravity: Vector2::new(0.0, 250.0),
        }
    }

    fn load_svg_file(
        &mut self,
        svg_path: &str,
        svg_instance_transform: Transformation<f32>,
        settings: &BodySettings<f32>,
    ) -> Vec<Body> {
        if let Ok((mut template, meta)) = load_svg(svg_path, settings) {
            template
                .iter_mut()
                .for_each(|template| template.transform(svg_instance_transform));
            self.meta_data.extend(meta);
            template
        } else {
            eprint!("Failed to load SVG data from path: {}. Ignoring.", svg_path);
            Vec::new()
        }
    }

    fn create_scene(
        &mut self,
        width: usize,
        height: usize,
        svg_settings: &BodySettings<f32>,
    ) -> Vec<Body> {
        let button_scale = width.min(height) as f32 / 6.0;
        let button_drop_off = height as f32 - button_scale * 2.0;
        let get_transform = |offset_x| Transformation {
            offset: Vector2::new(offset_x, button_drop_off),
            scale: button_scale,
            rotation_radians: 0.0,
        };

        let mut scene_bodies = Vec::new();
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
            let svg_instance = self.load_svg_file(svg_data, transform, svg_settings);
            for body in &svg_instance {
                self.interactive_bodies.insert(body.id, name.to_string());
            }
            scene_bodies.extend(svg_instance);
        }

        let cloud_settings =
            BodySettings::from_values(1.0, 0.01, false, 1.0, 1.0, 0.6, 0.8, 0.5, 5, 8, 2.0, 3);

        let mut cloud_bodies = self.load_svg_file(
            include_str!("../../assets/clouds_foreground.svg"),
            Transformation {
                offset: Vector2::new(0.0, height as f32 - width as f32 / 16.0),
                scale: width as f32,
                rotation_radians: 0.0,
            },
            &cloud_settings,
        );
        scene_bodies.append(&mut cloud_bodies);

        scene_bodies
    }

    fn handle_selection(&mut self, position: Vector2<f32>, time_stamp: f32) -> Vec<OutgoingEvent> {
        let mut outgoing_event_queue = Vec::new();
        for container in self.bodies.iter_mut() {
            for body in container.children.iter_mut() {
                if body.point_in_polygon(position) {
                    self.current_selection.push(body.id);
                    body.set_pinned(true);
                    if let Some(name) = self.interactive_bodies.get(&body.id) {
                        outgoing_event_queue.push(OutgoingEvent {
                            event_type: EventType::Selection,
                            body_id: body.id,
                            description: name.clone(),
                            time_stamp,
                        });
                    }
                }
            }
        }
        outgoing_event_queue
    }

    fn handle_deselection(&mut self, time_stamp: f32) -> Vec<OutgoingEvent> {
        let mut outgoing_event_queue = Vec::new();
        while let Some(selected_body) = self.current_selection.pop() {
            for container in self.bodies.iter_mut() {
                if let Some(child) = container.find_child_by_id_mut(selected_body) {
                    child.set_pinned(false);
                }
            }
            //todo add velocity to the body based on the average velocity of the mouse during the drag
            if let Some(name) = self.interactive_bodies.get(&selected_body) {
                outgoing_event_queue.push(OutgoingEvent {
                    event_type: EventType::Deselection,
                    body_id: selected_body,
                    description: name.clone(),
                    time_stamp,
                });
            }
        }
        outgoing_event_queue
    }

    fn handle_drag(
        &mut self,
        button: MouseButton,
        mouse_displacement: Vector2<f32>,
        _time_delta: f32,
    ) {
        if button != MouseButton::Left {
            return;
        }
        for container in self.bodies.iter_mut() {
            for selected_body in &self.current_selection {
                if let Some(child) = container.find_child_by_id_mut(*selected_body) {
                    child.set_uniform_movement(mouse_displacement, Vector2::zeros());
                }
            }
        }
    }
}
impl Game for PlaygroundGame {
    fn init(&mut self, width: usize, height: usize) {
        self.bodies.clear();
        self.meta_data.clear();
        self.current_selection.clear();
        self.interactive_bodies.clear();
        self.spawner.reset_runtime_state();
        self.spawner.update_settings(width, height);

        let svg_settings =
            BodySettings::from_values(1.0, 0.01, false, 0.5, 0.35, 0.3, 0.4, 0.5, 5, 8, 2.0, 3);
        let mut container_body = create_container(width, height, self.spawner.get_scale());

        container_body
            .children
            .extend(self.create_scene(width, height, &svg_settings));
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

    fn reset(&mut self, width: f32, height: f32) {
        self.init(width as usize, height as usize);
    }

    fn update(
        &mut self,
        mut incoming_events: VecDeque<Event>,
        mouse_speed: Option<Vector2<f32>>,
        dt_ms: f32,
    ) -> Vec<OutgoingEvent> {
        let mut outgoing_events = Vec::new();
        while let Some(event) = incoming_events.pop_front() {
            match event.event_type {
                MouseEventType::Down => {
                    if event.button == MouseButton::Left {
                        outgoing_events.extend(
                            self.handle_selection(
                                event.state.mouse_position,
                                event.state.time_stamp,
                            ),
                        );
                    }
                }
                MouseEventType::Up => {
                    if event.button == MouseButton::Left {
                        outgoing_events.extend(self.handle_deselection(event.state.time_stamp));
                    }
                }
                MouseEventType::Move => {
                    if event.button == MouseButton::Left {
                        if let Some(speed) = mouse_speed {
                            self.handle_drag(event.button, speed, dt_ms);
                        }
                    }
                }
            }
        }
        let dt: f32 = dt_ms / 1000.0;
        let settings = SolverSettings {
            reference_dt: 1.0 / 60.0,
            constraint_iterations: 8,
        };
        let capped_dt = dt.min(2.0 * settings.reference_dt);
        if let Some((body, meta)) = self.spawner.update(capped_dt) {
            self.meta_data.extend(meta);
            self.bodies[0].children.push(body);
        };

        for body in self.bodies.iter_mut() {
            let constant_force = chubby_bunny_core::force::constant_force(self.gravity);
            body.perform_step(&vec![constant_force], capped_dt, &settings);
        }

        outgoing_events
    }

    fn bodies_to_render(&self) -> &[Body] {
        &self.bodies
    }

    fn meta_data_to_render(&self) -> &HashMap<BodyId, BodyMeta> {
        &self.meta_data
    }

    fn current_selection_to_render(&self) -> &[BodyId] {
        &self.current_selection
    }
}

#[chubby_bunny_bindgen]
pub struct Playground(GameLoop<PlaygroundGame>);
