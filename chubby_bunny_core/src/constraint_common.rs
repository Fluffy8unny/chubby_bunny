use crate::{FloatingPointNumber, Particle};
use nalgebra::Vector2;
pub struct SolverSettings {
    pub reference_dt: f32,
    pub constraint_iterations: usize,
}

pub fn constraint_alpha_with_reference_dt<T: FloatingPointNumber>(
    stiffness: T,
    dt: T,
    settings: &SolverSettings,
) -> T {
    let alpha =
        stiffness * dt / T::from(settings.reference_dt * (settings.constraint_iterations as f32));
    alpha.clamp(T::zero(), T::one())
}

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

pub fn distribute_based_on_mass<T: FloatingPointNumber>(
    particle_a: &Particle<T>,
    particle_b: &Particle<T>,
) -> (T, T) {
    let total_mass = particle_a.mass + particle_b.mass;
    if total_mass <= T::zero() {
        return (T::from(0.5), T::from(0.5));
    }
    (particle_b.mass / total_mass, particle_a.mass / total_mass)
}
#[macro_export]
macro_rules! eps {
    ($type:ident, $exp:literal) => {
        // Evaluates 10^exp at compile-time as an f32, requires from32 trait
        <$type>::from(10.0_f32.powi(-$exp))
    };
}
