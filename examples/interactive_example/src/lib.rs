use chubby_bunny_bindgen::chubby_bunny_bindgen;
use chubby_bunny_canvas_renderer::game_loop::{Game, GameLoop};
use chubby_bunny_canvas_renderer::input::{Event, MouseButton, MouseEventType};
use chubby_bunny_canvas_renderer::js_types::{default_meta, EventType, OutgoingEvent};
use chubby_bunny_canvas_renderer::primitives::{create_polygon, SimpleBodySettings};
use chubby_bunny_core::{
    eps, Body, BodyId, CollisionConstraint, ExtrinsicConstraintType, FixedStepper, Particle,
    WallConstraint,
};
use chubby_bunny_svg::MetaMap;
use nalgebra::Vector2;
use std::collections::VecDeque;

struct InteractiveGame {
    bodies: Vec<Body>,
    meta_data: MetaMap,
    current_selection: Vec<BodyId>,
    stepper: FixedStepper,
}

impl InteractiveGame {
    pub fn new() -> Self {
        Self {
            bodies: Vec::new(),
            meta_data: MetaMap::new(),
            current_selection: Vec::new(),
            stepper: FixedStepper::default(),
        }
    }

    fn build_scene(center: Vector2<f32>, width: f32, height: f32) -> Body {
        let mut container_body = Body::empty();
        let mut create_particle_helper = |x, y| {
            container_body.particles.push(Particle::new(
                nalgebra::Vector2::new(x, y),
                nalgebra::Vector2::new(0.0, 0.0),
                1.0,
                0.1,
                true,
            ));
        };

        create_particle_helper(0.0, height);
        create_particle_helper(width, height);
        create_particle_helper(width, 0.0);
        create_particle_helper(0.0, 0.0);
        for i in 0..4 {
            container_body
                .children_constraints
                .push(ExtrinsicConstraintType::Global(Box::new(WallConstraint {
                    parent_point_idx_end: i,
                    parent_point_idx_origin: (i + 1) % 4,
                    stiffness: 1.0,
                })));
        }

        let distance_radius = width / 10.0;
        let poly_radius = distance_radius * 0.8;
        let settings = SimpleBodySettings {
            stiffness_distance: 0.5,
            stiffness_shear: 0.0,
            stiffness_area: 0.0,
            stiffness_bending: 0.0,
            friction: 0.002,
        };

        for i in (1..10).step_by(2) {
            let poly = create_polygon(
                Vector2::new(distance_radius * i as f32, center.y),
                poly_radius,
                12,
                &settings,
            );
            container_body.children.push(poly);
        }

        container_body.collision_constraint = Some(CollisionConstraint::new(0.99));
        container_body
    }
    fn handle_selection(&mut self, position: Vector2<f32>, time_stamp: f32) -> Vec<OutgoingEvent> {
        let mut outgoing_event_queue = Vec::new();
        for container in self.bodies.iter_mut() {
            for body in container.children.iter_mut() {
                if body.point_in_polygon(position) {
                    if !self.current_selection.contains(&body.id) {
                        self.current_selection.push(body.id);
                    }
                    body.set_pinned(true);
                    outgoing_event_queue.push(OutgoingEvent {
                        event_type: EventType::Selection,
                        body_id: body.id,
                        description: format!("Selected body with id: {}", body.id),
                        time_stamp,
                    });
                }
            }
        }
        outgoing_event_queue
    }

    fn handle_deselection(&mut self, time_stamp: f32) -> Vec<OutgoingEvent> {
        let mut outgoing_event_queue = Vec::new();
        for selected_body in self.current_selection.drain(..) {
            for container in self.bodies.iter_mut() {
                if let Some(child) = container.find_child_by_id_mut(selected_body) {
                    child.set_pinned(false);
                }
            }
            outgoing_event_queue.push(OutgoingEvent {
                event_type: EventType::Deselection,
                body_id: selected_body,
                description: format!("Deselected body with id: {}", selected_body),
                time_stamp,
            });
        }
        outgoing_event_queue
    }

    fn handle_drag(&mut self, _button: MouseButton, mouse_velocity: Vector2<f32>, time_delta: f32) {
        for container in self.bodies.iter_mut() {
            for selected_body in self.current_selection.iter() {
                if let Some(child) = container.find_child_by_id_mut(*selected_body) {
                    child.set_uniform_movement(mouse_velocity * time_delta, Vector2::zeros());
                }
            }
        }
    }
    fn handle_interaction(
        &mut self,
        mut incoming_events: VecDeque<Event>,
        dt_ms: f32,
    ) -> Vec<OutgoingEvent> {
        let mut outgoing_events = Vec::new();
        let mut movements = Vec::new();
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
                    if let Some(last_state) = event.last_state {
                        let displacement = event.state.mouse_position - last_state.mouse_position;
                        let time_delta = event.state.time_stamp - last_state.time_stamp;
                        if time_delta > eps!(f32, 6) {
                            movements.push(displacement / time_delta);
                        }
                    }
                }
            }
        }

        if !movements.is_empty() {
            let average_velocity =
                movements.iter().fold(Vector2::zeros(), |acc, &d| acc + d) / movements.len() as f32;
            self.handle_drag(MouseButton::Left, average_velocity, dt_ms);
        }
        outgoing_events
    }
}

impl Game for InteractiveGame {
    fn init(&mut self, width: usize, height: usize) {
        self.bodies.clear();
        self.meta_data.clear();

        let box_body = Self::build_scene(
            Vector2::new(width as f32 * 0.5, height as f32 * 0.5),
            width as f32,
            height as f32,
        );
        let mut container_meta = default_meta(box_body.id, 0);
        container_meta.smooth_edges = false;
        self.meta_data.insert(box_body.id, container_meta);
        self.bodies.push(box_body);
    }

    fn reset(&mut self, width: f32, height: f32) {
        self.init(width as usize, height as usize);
    }

    fn update(&mut self, incoming_events: VecDeque<Event>, dt_ms: f32) -> Vec<OutgoingEvent> {
        let outgoing_events = self.handle_interaction(incoming_events, dt_ms);
        let dt = dt_ms / 1000.0;
        let constant_force = chubby_bunny_core::force::constant_force(Vector2::new(0.0, 250.0)); //px/s^2
        self.stepper.advance(&mut self.bodies, &[constant_force], dt);
        outgoing_events
    }

    fn bodies_to_render(&self) -> &[Body] {
        &self.bodies
    }

    fn meta_data_to_render(&self) -> &MetaMap {
        &self.meta_data
    }
}

#[chubby_bunny_bindgen]
pub struct InteractiveExample(GameLoop<InteractiveGame>);
