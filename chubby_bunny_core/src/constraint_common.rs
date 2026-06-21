use crate::{FloatingPointNumber, Particle};
use nalgebra::Vector2;

/// Settings for the constraint solver:
/// The reference time step, that basically says what time is expected
/// The number of solver iterations to perform per frame. Higher values can improve stability but reduce performance.
pub struct SolverSettings {
    pub reference_dt: f32,
    pub constraint_iterations: usize,
}

/// Calculates the alpha value for a constraint based on its stiffness, the time step, and the solver settings.
/// This ensures that the constraint behaves consistently across varying frame times and solver iteration counts.
pub fn constraint_alpha_with_reference_dt<T: FloatingPointNumber>(
    stiffness: T,
    dt: T,
    settings: &SolverSettings,
) -> T {
    let alpha =
        stiffness * dt / T::from(settings.reference_dt * (settings.constraint_iterations as f32));
    alpha.clamp(T::zero(), T::one())
}

/// Calculates the correction vector needed to maintain a target distance between two particles.
///
/// Used in all distance based constraints, such as `DistanceConstraint` and `AttachmentConstraint`.
pub fn get_distance_correction_vector<T: FloatingPointNumber>(
    particle_a: &Particle<T>,
    particle_b: &Particle<T>,
    stiffness: T,
    target_distance: T,
    dt: T,
    solver_settings: &SolverSettings,
) -> Vector2<T> {
    let line_between = particle_b.position - particle_a.position;
    let point_distance = line_between.norm();
    if point_distance <= T::zero() {
        return Vector2::zeros();
    }
    let move_direction = line_between / point_distance;
    let alpha = constraint_alpha_with_reference_dt(stiffness, dt, solver_settings);

    let correction_magnitude = alpha * (target_distance - point_distance) / T::from(2.0);
    move_direction * correction_magnitude
}

/// Simple 2d normal, it's used all over the place
pub fn get_normal<T: FloatingPointNumber>(
    start: Vector2<T>,
    end: Vector2<T>,
) -> Option<Vector2<T>> {
    let edge_vector = end - start;
    if edge_vector.norm_squared() <= T::zero() {
        return None;
    }
    Some(Vector2::new(-edge_vector.y, edge_vector.x).normalize())
}
/// Distributes correction weights between two particles based on their masses,
///  so that heavier particles receive less correction and lighter particles receive more correction.
pub fn distribute_based_on_mass<T: FloatingPointNumber>(
    particle_a: &Particle<T>,
    particle_b: &Particle<T>,
    base_weight_a: T,
    base_weight_b: T,
) -> (T, T) {
    let base_weight_sum = base_weight_a + base_weight_b;
    let (base_weight_a, base_weight_b) = if base_weight_sum > T::zero() {
        (
            base_weight_a / base_weight_sum,
            base_weight_b / base_weight_sum,
        )
    } else {
        (T::from(0.5_f32), T::from(0.5_f32))
    };

    let total_mass = particle_a.mass + particle_b.mass;
    if total_mass <= T::zero() {
        return (base_weight_a, base_weight_b);
    }

    let mass_weight_a = particle_b.mass / total_mass;
    let mass_weight_b = particle_a.mass / total_mass;

    let raw_weight_a = base_weight_a * mass_weight_a;
    let raw_weight_b = base_weight_b * mass_weight_b;
    let raw_weight_sum = raw_weight_a + raw_weight_b;

    (raw_weight_a / raw_weight_sum, raw_weight_b / raw_weight_sum)
}

/// Macro to get an epsilon value as type T
/// The exponent is negative. For example, `eps!(f32, 4)` gives 0.0001 as an f32.
#[macro_export]
macro_rules! eps {
    ($type:ident, $exp:literal) => {
        // Evaluates 10^exp at compile-time as an f32, requires from32 trait
        <$type>::from(10.0_f32.powi(-$exp))
    };
}
