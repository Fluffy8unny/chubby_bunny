pub mod intrinsic_contraint;
pub use intrinsic_contraint::{AreaConstraint, DistanceConstraint, IntrinsicContraint};

pub mod body;
pub use body::{Body, BodyId, ExtrinsicConstraintType};

pub mod particle;
pub use particle::Particle;

pub mod force;
pub use force::{constant_force, Force};

pub mod extrinsic_constraint;
pub use extrinsic_constraint::{
    AttachmentConstraint, GlobalExtrinsicConstraint, LocalExtrinsicConstraint, WallConstraint,
};

mod constraint_common;
pub use constraint_common::SolverSettings;
