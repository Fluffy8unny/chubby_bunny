use chubby_bunny_bindgen::chubby_bunny_bindgen;
use chubby_bunny_canvas_renderer::game_loop::{Game, GameLoop};
use chubby_bunny_canvas_renderer::input::Event;
use chubby_bunny_canvas_renderer::js_types::{default_meta, OutgoingEvent};
use chubby_bunny_core::{
    Body, ExtrinsicConstraintType, Particle, SolverSettings, Transformation, WallConstraint,
};
use chubby_bunny_svg::{load_svg, BodySettings, MetaMap};
use nalgebra::Vector2;
use std::collections::VecDeque;
use web_sys::console;
struct SVGGame {
    bodies: Vec<Body>,
    meta_data: MetaMap,
}

impl SVGGame {
    pub fn new() -> Self {
        Self {
            bodies: Vec::new(),
            meta_data: MetaMap::new(),
        }
    }

    fn load_svg_file(
        &mut self,
        svg_source: &str,
        svg_instance_transform: Transformation<f32>,
        settings: &BodySettings<f32>,
    ) -> Vec<Body> {
        if let Ok((mut template, meta)) = load_svg(svg_source, settings) {
            template
                .iter_mut()
                .for_each(|template| template.transform(svg_instance_transform));
            self.meta_data.extend(meta);
            template
        } else {
            console::error_1(&"Failed to load SVG data. Ignoring.".into());
            Vec::new()
        }
    }

    fn build_scene(&mut self, center: Vector2<f32>, width: f32, height: f32) -> Body {
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

        create_particle_helper(0.0, height as f32 * 0.9);
        create_particle_helper(width as f32, height as f32 * 0.9);
        create_particle_helper(width as f32, 0.1 * height as f32);
        create_particle_helper(0.0, 0.1 * height as f32);
        for i in 0..4 {
            container_body
                .children_constraints
                .push(ExtrinsicConstraintType::Global(Box::new(WallConstraint {
                    parent_point_idx_end: i,
                    parent_point_idx_origin: (i + 1) % 4,
                    stiffness: 1.0,
                })));
        }
        let svg_settings =
            BodySettings::from_values(1.0, 0.01, false, 0.5, 0.35, 0.3, 0.4, 0.5, 5, 8, 2.0, 3);

        let test_svg_bodies = self.load_svg_file(
            include_str!("../web/assets/bunny.svg"),
            Transformation {
                offset: center,
                scale: height * 0.25,
                rotation_radians: 0.0,
            },
            &svg_settings,
        );

        container_body.children.extend(test_svg_bodies);
        container_body
    }
}

impl Game for SVGGame {
    fn init(&mut self, width: usize, height: usize) {
        self.bodies.clear();
        self.meta_data.clear();

        let box_body = self.build_scene(
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
        let settings = SolverSettings {
            reference_dt: 1.0 / 60.0,
            constraint_iterations: 6,
        };
        let dt = dt_ms / 1000.0;
        for body in self.bodies.iter_mut() {
            let constant_force = chubby_bunny_core::force::constant_force(Vector2::new(0.0, 250.0)); //px/s^2
            body.perform_step(&vec![constant_force], dt, &settings);
        }
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
pub struct SVGExample(GameLoop<SVGGame>);
