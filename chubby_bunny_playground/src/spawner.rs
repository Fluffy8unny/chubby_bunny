use chubby_bunny_core::{Body, BodyId, FloatingPointNumber, Transformation};
use chubby_bunny_svg::{instantiate_svg_body, load_svg, BodyMeta, BodySettings};

use nalgebra::Vector2;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::collections::HashMap;
struct RandomPicker<T> {
    items: Vec<T>,
    min_val: T,
    max_val: T,
    interval: T,
}

impl<T: FloatingPointNumber> RandomPicker<T> {
    pub fn new(items: Vec<T>, min_val: T, max_val: T, interval: T) -> Self {
        Self {
            items,
            min_val,
            max_val,
            interval,
        }
    }

    pub fn pick(&mut self) -> Option<T> {
        if self.items.is_empty() {
            let scale_half = self.interval / T::from(2.0);
            let mut val = self.min_val + scale_half;
            while val < self.max_val - scale_half {
                self.items.push(val);
                val += self.interval;
            }
            self.items.shuffle(&mut SmallRng::seed_from_u64(42));
        }
        self.items.pop()
    }
}
pub struct BunnySpawner<T = f32> {
    number_of_bunnies: usize,
    max_bunnies: usize,
    spawn_timer: f32,
    spawn_interval: f32,
    bunny_bodies: Vec<Body<T>>,
    bunny_meta: Vec<HashMap<BodyId, BodyMeta>>,
    min_pos_x: T,
    max_pos_x: T,
    min_scale: T,
    max_scale: T,
    svg_settings: Option<BodySettings<T>>,
    random_picker: RandomPicker<T>,
}

impl<T: FloatingPointNumber> BunnySpawner<T> {
    pub fn new(
        spawn_interval: f32,
        max_bunnies: usize,
        max_pos: T,
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
            min_scale,
            max_scale,
            svg_settings: None,
            random_picker: RandomPicker::new(Vec::new(), T::zero(), max_pos, max_scale),
        }
    }

    pub fn get_scale(&self) -> T {
        self.max_scale
    }

    fn spawn_bunny(&mut self) -> Option<(Body<T>, HashMap<BodyId, BodyMeta>)> {
        let xpos = self.random_picker.pick()?;
        let scale =
            T::from(rand::random::<f32>()) * (self.max_scale - self.min_scale) + self.min_scale;
        let rotation_radians = T::from(rand::random::<f32>()) * T::from(std::f32::consts::TAU);
        let svg_instance_transform = Transformation {
            offset: Vector2::new(xpos, -self.max_scale),
            scale,
            rotation_radians,
        };

        let picked_bunny: usize = rand::random_range(0..self.bunny_bodies.len());
        Some(instantiate_svg_body(
            self.bunny_bodies.get(picked_bunny)?,
            self.bunny_meta.get(picked_bunny)?,
            svg_instance_transform,
        ))
    }

    pub fn update(&mut self, dt_s: f32) -> Option<(Body<T>, HashMap<BodyId, BodyMeta>)> {
        let dt_ms = dt_s * 1000.0;
        if self.number_of_bunnies >= self.max_bunnies {
            return None;
        }
        self.spawn_timer += dt_ms;
        if self.spawn_timer >= self.spawn_interval {
            self.spawn_timer -= self.spawn_interval;
            self.number_of_bunnies += 1;
            Some(self.spawn_bunny()?)
        } else {
            None
        }
    }

    pub fn update_settings(&mut self, width: usize, height: usize) {
        self.max_scale = T::from_usize(width.min(height) / 8).unwrap();
        self.min_scale = T::from_usize(width.min(height) / 18).unwrap();
        self.min_pos_x = T::zero();
        self.max_pos_x = T::from_usize(width).unwrap() - self.max_scale;
        self.random_picker = RandomPicker::new(
            Vec::new(),
            self.min_pos_x,
            self.max_pos_x,
            self.max_scale * T::from(1.5),
        );
    }

    pub fn reset_runtime_state(&mut self) {
        self.number_of_bunnies = 0;
        self.spawn_timer = 0.0;
        self.random_picker = RandomPicker::new(
            Vec::new(),
            self.min_pos_x,
            self.max_pos_x,
            self.max_scale * T::from(1.5),
        );
    }

    pub fn load_bunnies_from_svg(&mut self, svg_data: Vec<&str>, settings: BodySettings<T>) {
        self.bunny_bodies.clear();
        self.bunny_meta.clear();
        self.svg_settings = Some(settings);
        for svg_path in svg_data.iter() {
            if let Ok((mut bodies, meta)) = load_svg(svg_path, self.svg_settings.as_ref().unwrap())
            {
                self.bunny_bodies.append(&mut bodies);
                self.bunny_meta.push(meta);
            } else {
                eprint!("Failed to load SVG data from path: {}. Ignoring.", svg_path);
            }
        }
    }
}
