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

#[derive(Clone)]
pub struct BendingConstraint<T> {
    pub idx_prev: usize,
    pub idx_center: usize,
    pub idx_next: usize,
    pub rest_angle: T,
    pub stiffness: T,
}

impl<T: FloatingPointNumber> BendingConstraint<T> {
    pub fn new(
        idx_prev: usize,
        idx_center: usize,
        idx_next: usize,
        particles: &[Particle<T>],
        stiffness: T,
    ) -> Self {
        let prev = particles[idx_prev].position;
        let center = particles[idx_center].position;
        let next = particles[idx_next].position;
        let v_prev = prev - center;
        let v_next = next - center;
        let rest_angle = (v_prev.x * v_next.y - v_prev.y * v_next.x).atan2(v_prev.dot(&v_next));
        Self {
            idx_prev,
            idx_center,
            idx_next,
            rest_angle,
            stiffness,
        }
    }

    fn wrap_angle_to_pi(mut angle: T) -> T {
        let pi = T::pi();
        let two_pi = pi * T::from(2.0);
        while angle > pi {
            angle -= two_pi;
        }
        while angle < -pi {
            angle += two_pi;
        }
        angle
    }
}

impl<T: FloatingPointNumber> IntrinsicConstraint<T> for BendingConstraint<T> {
    fn solve(&self, particles: &mut Vec<Particle<T>>, dt: T, solver_settings: &SolverSettings) {
        let prev = particles[self.idx_prev].position;
        let center = particles[self.idx_center].position;
        let next = particles[self.idx_next].position;

        let e_prev = prev - center;
        let e_next = next - center;

        let prev_len_sq = e_prev.norm_squared();
        let next_len_sq = e_next.norm_squared();
        if prev_len_sq <= T::from(1.0e-12_f32) || next_len_sq <= T::from(1.0e-12_f32) {
            return;
        }

        let current_angle = (e_prev.x * e_next.y - e_prev.y * e_next.x).atan2(e_prev.dot(&e_next));
        let c = Self::wrap_angle_to_pi(current_angle - self.rest_angle);
        if c.abs() <= T::from(1.0e-6_f32) {
            return;
        }

        let alpha = constraint_alpha_with_reference_dt(self.stiffness, dt, solver_settings);
        let c_scaled = c * alpha;

        let grad_prev = Vector2::new(e_prev.y, -e_prev.x) / prev_len_sq;
        let grad_next = Vector2::new(-e_next.y, e_next.x) / next_len_sq;
        let grad_center = -(grad_prev + grad_next);

        let denom =
            grad_prev.norm_squared() + grad_center.norm_squared() + grad_next.norm_squared();
        if denom <= T::from(1.0e-12_f32) {
            return;
        }

        let lambda = -c_scaled / denom;

        particles[self.idx_prev].apply_position_correction_to_particle(&(grad_prev * lambda));
        particles[self.idx_center].apply_position_correction_to_particle(&(grad_center * lambda));
        particles[self.idx_next].apply_position_correction_to_particle(&(grad_next * lambda));
    }
}
