use chubby_bunny_bindgen::chubby_bunny_bindgen;
use chubby_bunny_canvas_renderer::game_loop::{Game, GameLoop};
use chubby_bunny_canvas_renderer::input::Event;
use chubby_bunny_canvas_renderer::js_types::{default_meta, OutgoingEvent};
use chubby_bunny_canvas_renderer::primitives::{create_polygon, SimpleBodySettings};
use chubby_bunny_core::{
    Body, CollisionConstraint, ExtrinsicConstraintType, FixedStepper, Particle, WallConstraint,
};
use chubby_bunny_svg::MetaMap;
use nalgebra::Vector2;
use std::collections::VecDeque;

struct ConstraintsGame {
    bodies: Vec<Body>,
    meta_data: MetaMap,
    stepper: FixedStepper,
}

impl ConstraintsGame {
    pub fn new() -> Self {
        Self {
            bodies: Vec::new(),
            meta_data: MetaMap::new(),
            stepper: FixedStepper::default(),
        }
    }

    fn build_box(center: Vector2<f32>, width: f32, height: f32) -> Body {
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

        create_particle_helper(0.0, height * 0.9);
        create_particle_helper(width, height * 0.9);
        create_particle_helper(width, 0.1 * height);
        create_particle_helper(0.0, 0.1 * height);
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
        let poly_radius = distance_radius;

        let poly_distance_only = create_polygon(
            Vector2::new(distance_radius, center.y),
            poly_radius,
            12,
            &SimpleBodySettings {
                stiffness_distance: 0.5,
                stiffness_shear: 0.0,
                stiffness_area: 0.0,
                stiffness_bending: 0.0,
                friction: 0.72,
            },
        );

        let poly_distance_shear_only = create_polygon(
            Vector2::new(distance_radius * 3.0, center.y),
            poly_radius,
            12,
            &SimpleBodySettings {
                stiffness_distance: 0.5,
                stiffness_shear: 0.2,
                stiffness_area: 0.0,
                stiffness_bending: 0.0,
                friction: 0.72,
            },
        );

        let poly_distance_area = create_polygon(
            Vector2::new(distance_radius * 5.0, center.y),
            poly_radius,
            12,
            &SimpleBodySettings {
                stiffness_distance: 0.5,
                stiffness_shear: 0.0,
                stiffness_area: 0.5,
                stiffness_bending: 0.0,
                friction: 0.72,
            },
        );

        let poly_bending = create_polygon(
            Vector2::new(distance_radius * 7.0, center.y),
            poly_radius,
            12,
            &SimpleBodySettings {
                stiffness_distance: 0.5,
                stiffness_shear: 0.0,
                stiffness_area: 0.0,
                stiffness_bending: 0.3,
                friction: 0.72,
            },
        );

        let poly_stiff = create_polygon(
            Vector2::new(distance_radius * 9.0, center.y),
            poly_radius,
            12,
            &SimpleBodySettings {
                stiffness_distance: 0.5,
                stiffness_shear: 0.5,
                stiffness_area: 0.5,
                stiffness_bending: 0.5,
                friction: 0.72,
            },
        );

        container_body.children.push(poly_distance_only);
        container_body.children.push(poly_distance_shear_only);
        container_body.children.push(poly_distance_area);
        container_body.children.push(poly_bending);
        container_body.children.push(poly_stiff);
        container_body.collision_constraint = Some(CollisionConstraint::new(0.99));
        container_body
    }
}

impl Game for ConstraintsGame {
    fn init(&mut self, width: usize, height: usize) {
        self.bodies.clear();
        self.meta_data.clear();

        let box_body = Self::build_box(
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

    fn update(&mut self, _incoming_events: VecDeque<Event>, dt_ms: f32) -> Vec<OutgoingEvent> {
        let dt = dt_ms / 1000.0;
        let constant_force = chubby_bunny_core::force::constant_force(Vector2::new(0.0, 250.0)); //px/s^2
        self.stepper
            .advance(&mut self.bodies, &[constant_force], dt);
        Vec::new()
    }

    fn bodies_to_render(&self) -> &[Body] {
        &self.bodies
    }

    fn meta_data_to_render(&self) -> &MetaMap {
        &self.meta_data
    }
}

#[chubby_bunny_bindgen]
pub struct ConstraintsExample(GameLoop<ConstraintsGame>);
