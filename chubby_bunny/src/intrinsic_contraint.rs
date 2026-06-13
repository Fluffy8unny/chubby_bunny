use crate::constraint_common::{
    constraint_alpha_with_reference_dt, get_distance_correction_vector,
};
use crate::{FloatingPointNumber, Particle, SolverSettings};
use nalgebra::Vector2;

pub trait IntrinsicConstraint<T = f32> {
    fn solve(&self, particles: &mut Vec<Particle<T>>, dt: T, solver_settings: &SolverSettings);
}
#[derive(Clone)]
pub struct DistanceConstraint<T> {
    pub idx_left: usize,
    pub idx_right: usize,
    pub target_distance: T,
    pub stiffness: T,
}

impl<T: FloatingPointNumber> DistanceConstraint<T> {
    pub fn new(idx_left: usize, idx_right: usize, particles: &[Particle<T>], stiffness: T) -> Self {
        let target_distance = (particles[idx_right].position - particles[idx_left].position).norm();
        Self {
            idx_left,
            idx_right,
            target_distance,
            stiffness,
        }
    }
}

impl<T: FloatingPointNumber> IntrinsicConstraint<T> for DistanceConstraint<T> {
    fn solve(&self, particles: &mut Vec<Particle<T>>, dt: T, solver_settings: &SolverSettings) {
        let correction_vector = get_distance_correction_vector(
            &particles[self.idx_left],
            &particles[self.idx_right],
            self.stiffness,
            self.target_distance,
            dt,
            solver_settings,
        );
        particles[self.idx_left].apply_position_correction_to_particle(&(-correction_vector));
        particles[self.idx_right].apply_position_correction_to_particle(&correction_vector);
    }
}
#[derive(Clone)]
pub struct AreaConstraint<T> {
    pub idxs: Vec<usize>,
    pub rest_area: T,
    pub stiffness: T,
}

impl<T: FloatingPointNumber> AreaConstraint<T> {
    pub fn new(idxs: Vec<usize>, particles: &[Particle<T>], stiffness: T) -> Self {
        let rest_area = Self::calculate_area(&idxs, particles);
        Self {
            idxs,
            rest_area,
            stiffness,
        }
    }

    fn calculate_area(idxs: &[usize], particles: &[Particle<T>]) -> T {
        let mut area = T::zero();
        for i in 0..idxs.len() {
            let current = &particles[idxs[i]];
            let next = &particles[idxs[(i + 1) % idxs.len()]];
            //det form of trapazoidal rule ad-bc
            area += current.position.x * next.position.y - next.position.x * current.position.y;
        }
        area.abs() / T::from(2.0)
    }
}

impl<T: FloatingPointNumber> IntrinsicConstraint<T> for AreaConstraint<T> {
    fn solve(&self, particles: &mut Vec<Particle<T>>, dt: T, solver_settings: &SolverSettings) {
        let current_area = Self::calculate_area(&self.idxs, particles);
        if current_area <= T::zero() {
            return;
        }

        let n = T::from(self.idxs.len() as f32);
        let centroid = self
            .idxs
            .iter()
            .fold(Vector2::zeros(), |acc, &i| acc + particles[i].position)
            / n;

        let scale_correction = (self.rest_area / current_area).sqrt() - T::one();
        let alpha = constraint_alpha_with_reference_dt(self.stiffness, dt, solver_settings);

        for idx in &self.idxs {
            let offset = particles[*idx].position - centroid;
            particles[*idx]
                .apply_position_correction_to_particle(&(offset * scale_correction * alpha));
        }
    }
}
