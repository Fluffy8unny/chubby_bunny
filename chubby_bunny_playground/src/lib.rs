mod primitives;
pub use primitives::{create_polygon, create_quad, create_rect}; //keeping this pub to surpress warnings atm

mod input;
mod spawner;

pub mod js_types;
pub mod playground;
pub use playground::Playground;
