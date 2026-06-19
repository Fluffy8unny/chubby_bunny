mod meta;
mod settings;
mod svg_constraints;
mod svg_parser;

pub use meta::{BodyMeta, Color, MetaMap};
pub use settings::{AttachmentSettings, BodySettings, ConstraintSettings, ParticleSettings};
pub use svg_parser::{instantiate_svg_bodies, instantiate_svg_body, load_svg};
