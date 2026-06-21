mod meta;
mod settings;
mod svg_constraints;
mod svg_parser;

pub use meta::{BodyMeta, Color, MetaMap};
pub use settings::{
    AttachmentSettings, BodySettings, ConstraintSettings, ParticleSettings, SVGConstraintSettings,
};
pub use svg_parser::{
    add_automatic_constraints, instantiate_svg_bodies, instantiate_svg_body, load_svg,
    load_svg_to_body,
};
