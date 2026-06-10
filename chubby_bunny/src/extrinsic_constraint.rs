use crate::constraint_common::get_distance_correction_vector;
use crate::{Body, BodyId, Number, Particle, SolverSettings};
use nalgebra::Vector2;
pub enum ExtrinsicConstraintType<T> {
    Global(Box<dyn GlobalExtrinsicConstraint<T>>),
    Local(Box<dyn LocalExtrinsicConstraint<T>>),
}
pub trait GlobalExtrinsicConstraint<T = f32> {
    fn solve(
        &self,
        bodies: &mut Vec<Body<T>>,
        parent_particles: &[Particle<T>],
        dt: T,
        solver_settings: &SolverSettings,
    );
}
pub trait LocalExtrinsicConstraint<T = f32> {
    fn solve(
        &self,
        body: &mut Body<T>,
        parent_particles: &[Particle<T>],
        dt: T,
        solver_settings: &SolverSettings,
    );
    fn get_id(&self) -> BodyId;
}

pub struct WallConstraint<T> {
    pub parent_point_idx_origin: usize,
    pub parent_point_idx_end: usize,
    pub stiffness: T,
}

impl<T: Number> GlobalExtrinsicConstraint<T> for WallConstraint<T> {
    fn solve(
        &self,
        bodies: &mut Vec<Body<T>>,
        parent_particles: &[Particle<T>],
        dt: T,
        _solver_settings: &SolverSettings,
    ) {
        for body in bodies.iter_mut() {
            //calculate line based on parent points
            let line_origin = parent_particles[self.parent_point_idx_origin].position;
            let line_end = parent_particles[self.parent_point_idx_end].position;
            let line_direction = line_end - line_origin;
            if line_direction.norm_squared() <= T::zero() {
                return;
            }

            let line_normal = Vector2::new(-line_direction.y, line_direction.x).normalize();
            let eps = T::from(1.0e-4_f32);
            for particle in body.particles.iter_mut().filter(|p| !p.pinned) {
                let to_particle = particle.position - line_origin;
                let distance = to_particle.dot(&line_normal);
                if distance < T::zero() {
                    let correction_vector = line_normal * (-distance + eps);
                    particle.apply_position_correction(&correction_vector);
                    //we're cheating a bit here, but it's fine for velet integration.
                    let normal_velocity = particle.velocity.dot(&line_normal);

                    //prevent this from being changed more than once per framer
                    if normal_velocity < T::zero() {
                        let reflected_velocity =
                            particle.velocity - line_normal * normal_velocity * (T::from(2.0_f32));
                        particle.pre_integration_position =
                            particle.position - reflected_velocity * dt * self.stiffness;
                    }
                }
            }
        }
    }
}

pub struct AttachmentConstraint<T> {
    pub id: BodyId,
    pub point_idxs_parent: Vec<usize>,
    pub point_idxs_child: Vec<usize>,
    pub target_distances: Vec<T>,
    pub stiffness: T,
}
impl<T: Number> AttachmentConstraint<T> {
    pub fn new(
        body_id: BodyId,
        parent: &Body<T>,
        child: &Body<T>,
        point_idxs_parent: Vec<usize>,
        point_idxs_child: Vec<usize>,
        stiffness: T,
    ) -> Self {
        assert_eq!(
            point_idxs_parent.len(),
            point_idxs_child.len(),
            "Parent and child point index lists must be of the same length"
        );
        let mut target_distances = vec![T::zero(); point_idxs_parent.len()];
        for (i, (parent_idx, child_idx)) in point_idxs_parent
            .iter()
            .zip(point_idxs_child.iter())
            .enumerate()
        {
            let parent_particle = &parent.particles[*parent_idx];
            let child_particle = &child.particles[*child_idx];
            target_distances[i] = (parent_particle.position - child_particle.position).norm();
        }
        Self {
            id: body_id,
            point_idxs_parent,
            point_idxs_child,
            target_distances,
            stiffness,
        }
    }
}
impl<T: Number> LocalExtrinsicConstraint<T> for AttachmentConstraint<T> {
    fn get_id(&self) -> BodyId {
        self.id
    }

    fn solve(
        &self,
        body: &mut Body<T>,
        parent_particles: &[Particle<T>],
        _dt: T,
        _solver_settings: &SolverSettings,
    ) {
        for ((parent_idx, child_idx), target_distance) in self
            .point_idxs_parent
            .iter()
            .zip(self.point_idxs_child.iter())
            .zip(self.target_distances.iter())
        {
            let parent_particle = &parent_particles[*parent_idx];
            let child_particle = &body.particles[*child_idx];

            let correction_vector = get_distance_correction_vector(
                parent_particle,
                child_particle,
                self.stiffness,
                *target_distance,
                _dt,
                _solver_settings,
            );
            body.particles[*child_idx].apply_position_correction(&correction_vector);
        }
    }
}
