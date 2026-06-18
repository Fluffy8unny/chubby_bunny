use crate::constraint_common::{
    constraint_alpha_with_reference_dt, get_distance_correction_vector,
};
use crate::{eps,FloatingPointNumber, Particle, SolverSettings, Transformation};
use dyn_clone::DynClone;
use nalgebra::Vector2;

pub trait IntrinsicConstraint<T = f32>: DynClone {
    fn solve(&self, particles: &mut Vec<Particle<T>>, dt: T, solver_settings: &SolverSettings);
    fn transform_params(&mut self, _transformation: Transformation<T>) {}
}
dyn_clone::clone_trait_object!(<T> IntrinsicConstraint<T>);
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

    fn transform_params(&mut self, transformation: Transformation<T>) {
        self.target_distance *= transformation.scale;
    }
}
#[derive(Clone)]
pub struct AreaConstraint<T> {
    pub rest_area: T,
    pub stiffness: T,
}

impl<T: FloatingPointNumber> AreaConstraint<T> {
    pub fn new(particles: &[Particle<T>], stiffness: T) -> Self {
        let rest_area = Self::calculate_signed_area(particles);
        Self {
            rest_area,
            stiffness,
        }
    }

    fn calculate_signed_area( particles: &[Particle<T>]) -> T {
        let mut area = T::zero();
        for i in 0..particles.len() {
            let current = &particles[i];
            let next = &particles[(i + 1) % particles.len()];
            //det form of trapazoidal rule ad-bc
            area += current.position.x * next.position.y - next.position.x * current.position.y;
        }
        area / T::from(2.0)
    }
}

impl<T: FloatingPointNumber> IntrinsicConstraint<T> for AreaConstraint<T> {
    fn solve(&self, particles: &mut Vec<Particle<T>>, dt: T, solver_settings: &SolverSettings) {
        if particles.len() < 3 {
            return;
        }

        let current_area = Self::calculate_signed_area(particles);
        let c = current_area - self.rest_area;
        if c.abs() <= eps!(T, 8) {
            return;
        }

        let alpha = constraint_alpha_with_reference_dt(self.stiffness, dt, solver_settings);
        if alpha <= T::zero() {
            return;
        }

        let n = particles.len();
        let half = T::from(0.5_f32);
        let mut grads = Vec::with_capacity(n);
        let mut gradient_sum = T::zero();

        for i in 0..n {
            let prev = particles[(i + n - 1) % n].position;
            let next = particles[(i + 1) % n].position;
            let grad = Vector2::new((next.y - prev.y) * half, (prev.x - next.x) * half); //normal
            gradient_sum += grad.norm_squared();
            grads.push(grad);
        }

        if gradient_sum <= eps!(T, 12) {
            return;
        }

        let lambda = -alpha * c / gradient_sum;
        for i in 0..n {
            particles[i].apply_position_correction_to_particle(&(grads[i] * lambda));
        }
    }

    fn transform_params(&mut self, transformation: Transformation<T>) {
        self.rest_area *= transformation.scale * transformation.scale;
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
    pub fn new(idx_center: usize, particles: &[Particle<T>], stiffness: T) -> Self {
        let n = particles.len();
        let idx_prev = (idx_center + n - 1) % n;
        let idx_next = (idx_center + 1) % n;

        let prev = particles[idx_prev].position;
        let center = particles[idx_center].position;
        let next = particles[idx_next].position;
        let (v_prev, v_next) = Self::get_edges(prev, center, next);
        let rest_angle = Self::calculate_angle(v_prev, v_next);
        Self {
            idx_prev,
            idx_center,
            idx_next,
            rest_angle,
            stiffness,
        }
    }

    fn calculate_angle(v_prev: Vector2<T>, v_next: Vector2<T>) -> T {
        (v_prev.x * v_next.y - v_prev.y * v_next.x).atan2(v_prev.dot(&v_next))
    }

    fn get_edges(prev: Vector2<T>, center: Vector2<T>, next: Vector2<T>) -> (Vector2<T>, Vector2<T>) {
        (prev - center, next - center)
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

        let (v_prev, v_next) = Self::get_edges(prev, center, next);

        let prev_len_sq = v_prev.norm_squared();
        let next_len_sq = v_next.norm_squared();
        if prev_len_sq <= eps!(T, 12) || next_len_sq <= eps!(T, 12) {
            return;
        }

        let current_angle = Self::calculate_angle(v_prev, v_next);
        let c = Self::wrap_angle_to_pi(current_angle - self.rest_angle);
        if c.abs() <= eps!(T, 6) {
            return;
        }

        let alpha = constraint_alpha_with_reference_dt(self.stiffness, dt, solver_settings);
        let c_scaled = c * alpha;

        let grad_prev = Vector2::new(v_prev.y, -v_prev.x) / prev_len_sq;
        let grad_next = Vector2::new(-v_next.y, v_next.x) / next_len_sq;
        let grad_center = -(grad_prev + grad_next);

        let denom =
            grad_prev.norm_squared() + grad_center.norm_squared() + grad_next.norm_squared();
        if denom <= eps!(T, 12) {
            return;
        }

        let lambda = -c_scaled / denom;

        particles[self.idx_prev].apply_position_correction_to_particle(&(grad_prev * lambda));
        particles[self.idx_center].apply_position_correction_to_particle(&(grad_center * lambda));
        particles[self.idx_next].apply_position_correction_to_particle(&(grad_next * lambda));
    }
}
