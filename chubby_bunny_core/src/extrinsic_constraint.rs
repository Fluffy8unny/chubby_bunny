use crate::constraint_common::{get_distance_correction_vector, get_normal};
use crate::{eps, Body, BodyId, FloatingPointNumber, Particle, SolverSettings, Transformation};
use dyn_clone::DynClone;
use std::collections::HashMap;

/// Constraint that acts on particles of multiple bodies, where the bodies influence each other.
#[derive(Clone)]
pub enum ExtrinsicConstraintType<T> {
    /// Constraint that acts on all bodies in the system, such as collision with a global boundary.
    Global(Box<dyn GlobalExtrinsicConstraint<T>>),
    /// Constraint that acts on a single body, but depends on the state of other bodies, such as an attachment.
    Local(Box<dyn LocalExtrinsicConstraint<T>>),
}
/// Constraints that act on particles of multiple bodies, where the bodies influence each other.
pub trait GlobalExtrinsicConstraint<T = f32>: DynClone {
    fn solve(
        &self,
        bodies: &mut Vec<Body<T>>,
        parent_particles: &[Particle<T>],
        dt: T,
        solver_settings: &SolverSettings,
    );
}
dyn_clone::clone_trait_object!(<T> GlobalExtrinsicConstraint<T>);

/// Constraints that act on a single body, but depend on the state of other bodies.
pub trait LocalExtrinsicConstraint<T = f32>: DynClone {
    fn solve(
        &self,
        body: &mut Body<T>,
        parent_particles: &[Particle<T>],
        dt: T,
        solver_settings: &SolverSettings,
    );
    fn get_id(&self) -> BodyId;
    fn remap_body_ids(&mut self, _id_map: &HashMap<BodyId, BodyId>) {}
    fn transform_params(&mut self, _transformation: Transformation<T>) {}
}
dyn_clone::clone_trait_object!(<T> LocalExtrinsicConstraint<T>);

/// Collision between bodies, that are the children of a given parent body, and a line defined by two particles of the parent body.
///
/// The line segment is treated as a one-sided wall. The difference to the CollisionConstraint is, that the wall is completly static
/// This is mainly used to keep the whole scene on the screen by attaching all bodies to a large container body, that has wall constraints on its edges.
#[derive(Clone)]
pub struct WallConstraint<T> {
    /// Index of the parent particle that defines the line start.
    pub parent_point_idx_origin: usize,
    /// Index of the parent particle that defines the line end.
    pub parent_point_idx_end: usize,
    /// Solver stiffness in `[0, 1]` where higher values enforce the target more strongly.
    pub stiffness: T,
}

impl<T: FloatingPointNumber> GlobalExtrinsicConstraint<T> for WallConstraint<T> {
    fn solve(
        &self,
        bodies: &mut Vec<Body<T>>,
        parent_particles: &[Particle<T>],
        _dt: T,
        _solver_settings: &SolverSettings,
    ) {
        crate::profile_scope!("WallConstraint::solve");
        for body in bodies.iter_mut() {
            let line_origin = parent_particles[self.parent_point_idx_origin].position;
            let line_end = parent_particles[self.parent_point_idx_end].position;

            if let Some(line_normal) = get_normal(line_origin, line_end) {
                for particle in body.particles.iter_mut().filter(|p| !p.pinned) {
                    let to_particle = particle.position - line_origin;
                    let distance = to_particle.dot(&line_normal);
                    if distance < T::zero() {
                        let correction_vector = line_normal * (-distance + eps!(T, 4));
                        particle.apply_position_correction_to_particle(&correction_vector);
                    }
                }
            }
        }
    }
}

/// Attachment between a child body and a parent body, that tries to preserve the initial distance between defined pairs of parent and child particles.
#[derive(Clone)]
pub struct AttachmentConstraint<T> {
    /// Id of the parent body this constraint is attached to.
    pub id: BodyId,
    /// Indices of the parent particles involved in the attachment.
    pub point_idxs_parent: Vec<usize>,
    /// Indices of the child particles involved in the attachment.
    pub point_idxs_child: Vec<usize>,
    /// Target distances between corresponding parent and child particles.
    pub target_distances: Vec<T>,
    /// Solver stiffness in `[0, 1]` where higher values enforce the target more strongly.
    pub stiffness: T,
}

impl<T: FloatingPointNumber> AttachmentConstraint<T> {
    /// Builds an attachment constraint from the current distances between the specified parent and child particles.
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

        let target_distances = point_idxs_parent
            .iter()
            .zip(point_idxs_child.iter())
            .map(|(parent_idx, child_idx)| {
                let parent_particle = &parent.particles[*parent_idx];
                let child_particle = &child.particles[*child_idx];
                (child_particle.position - parent_particle.position).norm()
            })
            .collect();
        Self {
            id: body_id,
            point_idxs_parent,
            point_idxs_child,
            target_distances,
            stiffness,
        }
    }
}
impl<T: FloatingPointNumber> LocalExtrinsicConstraint<T> for AttachmentConstraint<T> {
    fn get_id(&self) -> BodyId {
        self.id
    }

    fn remap_body_ids(&mut self, id_map: &HashMap<BodyId, BodyId>) {
        if let Some(new_id) = id_map.get(&self.id) {
            self.id = *new_id;
        }
    }

    fn transform_params(&mut self, transformation: Transformation<T>) {
        for target_distance in self.target_distances.iter_mut() {
            *target_distance *= transformation.scale;
        }
    }

    fn solve(
        &self,
        body: &mut Body<T>,
        parent_particles: &[Particle<T>],
        dt: T,
        solver_settings: &SolverSettings,
    ) {
        crate::profile_scope!("AttachmentConstraint::solve");
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
                dt,
                solver_settings,
            );
            body.particles[*child_idx].apply_position_correction_to_particle(&correction_vector);
        }
    }
}
