pub mod intrinsic_contraint;
pub use intrinsic_contraint::{
    AreaConstraint, BendingConstraint, DistanceConstraint, IntrinsicConstraint,
};

pub mod body;
pub use body::{Body, BodyId, BoundingBox, Transformation};

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

/// Opens a profiling scope when the `profiling` feature is enabled.
///
/// This macro compiles to a no-op when `profiling` is disabled.
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {{
        #[cfg(feature = "profiling")]
        let _profile_guard = ::tracing::info_span!($name).entered();
    }};
}

pub trait FloatingPointNumber: nalgebra::RealField + Copy + From<f32> {}
impl<T> FloatingPointNumber for T where T: nalgebra::RealField + Copy + From<f32> {}
