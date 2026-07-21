use crate::{FloatingPointNumber, Particle};
use nalgebra::Vector2;

/// Clamps a raw stiffness into the `[0, 1]` fraction of constraint error removed per substep.
#[inline]
pub fn limited_stiffness<T: FloatingPointNumber>(stiffness: T) -> T {
    stiffness.clamp(T::zero(), T::one())
}

/// Calculates the correction vector needed to maintain a target distance between two particles.
///
/// Used in all distance based constraints, such as `DistanceConstraint` and `AttachmentConstraint`.
pub fn get_distance_correction_vector<T: FloatingPointNumber>(
    particle_a: &Particle<T>,
    particle_b: &Particle<T>,
    stiffness: T,
    target_distance: T,
) -> Vector2<T> {
    let line_between = particle_b.position - particle_a.position;
    let point_distance = line_between.norm();
    if point_distance <= T::zero() {
        return Vector2::zeros();
    }
    let move_direction = line_between / point_distance;

    let correction_magnitude = stiffness * (target_distance - point_distance) / T::from(2.0);
    move_direction * correction_magnitude
}

/// Simple 2d normal, it's used all over the place
#[inline]
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

/// Inverse mass used to weight position corrections. Mass is asserted positive at particle creation.
#[inline]
pub fn inverse_mass<T: FloatingPointNumber>(particle: &Particle<T>) -> T {
    T::one() / particle.mass
}

/// Distributes correction weights between two particles based on their inverse masses,
#[inline]
pub fn distribute_based_on_mass<T: FloatingPointNumber>(
    particle_a: &Particle<T>,
    particle_b: &Particle<T>,
    base_weight_a: T,
    base_weight_b: T,
) -> (T, T) {
    let raw_weight_a = base_weight_a * inverse_mass(particle_a);
    let raw_weight_b = base_weight_b * inverse_mass(particle_b);
    // Inverse masses are strictly positive
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
