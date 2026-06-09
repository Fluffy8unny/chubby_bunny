struct SolverSettings{
    pub reference_dt: f32,
    pub constraint_iterations: usize,
}

pub fn constraint_alpha_with_reference_dt<T>( stiffness: T, dt: T, reference_dt: T) -> T
where
    T: nalgebra::RealField + Copy + From<f32>,
{
    let alpha = stiffness * dt / reference_dt;
    alpha.clamp(T::zero(), T::one())
}   