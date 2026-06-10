pub mod intrinsic_contraint;
pub use intrinsic_contraint::{AreaConstraint, DistanceConstraint, IntrinsicContraint};

pub mod body;
pub use body::{Body, BodyId};

pub mod particle;
pub use particle::Particle;

pub mod force;
pub use force::{constant_force, Force};

pub mod extrinsic_constraint;
pub use extrinsic_constraint::{
    AttachmentConstraint, ExtrinsicConstraintType, GlobalExtrinsicConstraint,
    LocalExtrinsicConstraint, WallConstraint,
};
pub mod collision_constraint;
pub use collision_constraint::CollisionConstraint;

mod constraint_common;
pub use constraint_common::SolverSettings;

pub trait Number: nalgebra::RealField + Copy + From<f32> {}
impl<T> Number for T where T: nalgebra::RealField + Copy + From<f32> {}
