use crate::constraint_common::{constraint_alpha_with_reference_dt, distribute_based_on_mass};
use crate::{eps, FloatingPointNumber, Particle, SolverSettings, Transformation};
use dyn_clone::DynClone;
use nalgebra::Vector2;

/// Constraint that acts only on particles belonging to the same body.
///
/// Implementors typically preserve an intrinsic geometric property such as
/// edge length, enclosed area, or local bending angle.
pub trait IntrinsicConstraint<T = f32>: DynClone {
    /// Applies one constraint projection step to the provided particle set.
    ///
    /// `dt` (time delta between frames) and `solver_settings` are used to scale the internal parameters
    /// consistently across varying frame times.
    fn solve(&self, particles: &mut Vec<Particle<T>>, dt: T, solver_settings: &SolverSettings);

    /// Updates stored rest parameters after a geometric transformation.
    ///
    /// The default implementation is a no-op
    fn transform_params(&mut self, _transformation: Transformation<T>) {}
}
dyn_clone::clone_trait_object!(<T> IntrinsicConstraint<T>);

/// Preserves the distance between two particles.
#[derive(Clone)]
pub struct DistanceConstraint<T> {
    /// Index of the first particle.
    pub idx_left: usize,
    /// Index of the second particle.
    pub idx_right: usize,
    /// Rest distance between particles measured at construction time.
    pub target_distance: T,
    /// Solver stiffness in `[0, 1]` where higher values enforce the target more strongly.
    pub stiffness: T,

    pub weight_factors: (T, T),
}

impl<T: FloatingPointNumber> DistanceConstraint<T> {
    /// Builds a distance constraint from two particle indices and current positions.
    pub fn new(idx_left: usize, idx_right: usize, particles: &[Particle<T>], stiffness: T) -> Self {
        assert_ne!(idx_left, idx_right, "DistanceConstraint requires two distinct particle indices");

        let target_distance = (particles[idx_right].position - particles[idx_left].position).norm();
        let (weight_left, weight_right) =
            distribute_based_on_mass(&particles[idx_left], &particles[idx_right], T::from(0.5_f32), T::from(0.5_f32));

        Self {
            idx_left,
            idx_right,
            target_distance,
            stiffness,
            weight_factors: (weight_left, weight_right),
        }
    }
}

impl<T: FloatingPointNumber> IntrinsicConstraint<T> for DistanceConstraint<T> {
    fn solve(&self, particles: &mut Vec<Particle<T>>, dt: T, solver_settings: &SolverSettings) {
        crate::profile_scope!("DistanceConstraint::solve");
        let alpha = constraint_alpha_with_reference_dt(self.stiffness, dt, solver_settings);
        if alpha <= T::zero() {
            return;
        }

        let [left, right] = particles
            .get_disjoint_mut([self.idx_left, self.idx_right])
            .unwrap();//we checked at construction time that the indices are distinct, so this unwrap is safe

        if left.pinned && right.pinned {
            return;
        }

        let line_between = right.position - left.position;
        let point_distance = line_between.norm();
        if point_distance <= T::zero() {
            return;
        }

        let distance_delta = self.target_distance - point_distance;
        let correction = line_between * (alpha * distance_delta / point_distance);

        left.apply_position_correction_to_particle(&(-correction * self.weight_factors.0));
        right.apply_position_correction_to_particle(&(correction * self.weight_factors.1));
    }

    fn transform_params(&mut self, transformation: Transformation<T>) {
        self.target_distance *= transformation.scale;
    }
}

/// Preserves polygon area for a closed particle loop.
/// This is done by applying a correction to each particle in normal direction, to expan or contract the polygon
#[derive(Clone)]
pub struct AreaConstraint<T> {
    ///Area captured at construction time.
    pub rest_area: T,
    /// Solver stiffness in `[0, 1]` where higher values enforce the target more strongly.
    pub stiffness: T,
}

impl<T: FloatingPointNumber> AreaConstraint<T> {
    /// Builds an area constraint using the current polygon area.
    pub fn new(particles: &[Particle<T>], stiffness: T) -> Self {
        let rest_area = Self::calculate_signed_area(particles);
        Self {
            rest_area,
            stiffness,
        }
    }

    /// Computes signed polygon area
    fn calculate_signed_area(particles: &[Particle<T>]) -> T {
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
        crate::profile_scope!("AreaConstraint::solve");
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

/// Preserves the turning angle at a polygon vertex.
/// This is done by applying corrections to the center vertex and its two neighbors (prev,next).
#[derive(Clone)]
pub struct BendingConstraint<T> {
    /// Index of the previous neighbor vertex.
    pub idx_prev: usize,
    /// Index of the constrained center vertex.
    pub idx_center: usize,
    /// Index of the next neighbor vertex.
    pub idx_next: usize,
    /// Rest turning angle (radians) captured at construction time.
    pub rest_angle: T,
    /// Solver stiffness in `[0, 1]` where higher values enforce the target more strongly.
    pub stiffness: T,
}

impl<T: FloatingPointNumber> BendingConstraint<T> {
    /// Builds a bending constraint around `idx_center` and its ring neighbors.
    pub fn new(idx_center: usize, particles: &[Particle<T>], stiffness: T) -> Self {
        let n = particles.len();
        assert!(n >= 3, "BendingConstraint requires at least 3 particles to have distinct prev/center/next neighbors");
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

    /// Returns signed angle between two edge vectors.
    fn calculate_angle(v_prev: Vector2<T>, v_next: Vector2<T>) -> T {
        (v_prev.x * v_next.y - v_prev.y * v_next.x).atan2(v_prev.dot(&v_next))
    }

    /// Returns edge vectors incident to the center vertex.
    fn get_edges(
        prev: Vector2<T>,
        center: Vector2<T>,
        next: Vector2<T>,
    ) -> (Vector2<T>, Vector2<T>) {
        (prev - center, next - center)
    }

    /// Normalizes angle to `[-pi, pi]`.
    fn wrap_angle_to_pi(angle: T) -> T {
        let pi = T::pi();
        let two_pi = pi * T::from(2.0);
        // The angle can't be more than 2 pi away from the range, so we can just add/subtract 2 pi once to bring it into range.
        if angle > pi {
            angle - two_pi
        } else if angle < -pi {
            angle + two_pi
        } else {
            angle
        }
    }
}

impl<T: FloatingPointNumber> IntrinsicConstraint<T> for BendingConstraint<T> {
    fn solve(&self, particles: &mut Vec<Particle<T>>, dt: T, solver_settings: &SolverSettings) {
        crate::profile_scope!("BendingConstraint::solve");
        let [prev_particle, center_particle, next_particle] = particles
            .get_disjoint_mut([self.idx_prev, self.idx_center, self.idx_next])
            .unwrap(); //we checked at construction time that the indices are distinct, so this unwrap is safe

        let prev = prev_particle.position;
        let center = center_particle.position;
        let next = next_particle.position;

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

        prev_particle.apply_position_correction_to_particle(&(grad_prev * lambda));
        center_particle.apply_position_correction_to_particle(&(grad_center * lambda));
        next_particle.apply_position_correction_to_particle(&(grad_next * lambda));
    }
}
