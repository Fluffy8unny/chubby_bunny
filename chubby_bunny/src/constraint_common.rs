pub struct SolverSettings {
    pub reference_dt: f32,
    pub constraint_iterations: usize,
}

pub fn constraint_alpha_with_reference_dt<T>(stiffness: T, dt: T, settings: &SolverSettings) -> T
where
    T: nalgebra::RealField + Copy + From<f32>,
{
    let alpha =
        stiffness * dt / T::from(settings.reference_dt * (settings.constraint_iterations as f32));
    alpha.clamp(T::zero(), T::one())
}
