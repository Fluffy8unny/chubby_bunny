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
