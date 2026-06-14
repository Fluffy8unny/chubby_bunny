use chubby_bunny_core::{Body, BodyId, FloatingPointNumber, Transformation};
use chubby_bunny_svg::{instantiate_svg_body, load_svg, BodyMeta, BodySettings};

use nalgebra::Vector2;
use std::collections::HashMap;

pub struct BunnySpawner<T = f32> {
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
