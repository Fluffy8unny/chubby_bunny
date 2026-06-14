mod meta;
mod svg;
mod svg_constraints;

pub use meta::{BodyMeta, Color};
pub use svg::{
    instantiate_svg_bodies, instantiate_svg_body, load_svg, AttachmentSettings, BodySettings,
    ConstraintSettings, ParticleSettings,
};
