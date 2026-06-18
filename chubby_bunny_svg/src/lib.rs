mod meta;
mod svg_parser;
mod svg_constraints;

pub use meta::{BodyMeta, Color};
pub use svg_parser::{
    instantiate_svg_bodies, instantiate_svg_body, load_svg, AttachmentSettings, BodySettings,
    ConstraintSettings, ParticleSettings,
};
