use crate::collision_constraint::CollisionConstraint;
use crate::{
    ExtrinsicConstraintType, FloatingPointNumber, Force, IntrinsicConstraint, Particle,
    SolverSettings,
};
use itertools::Itertools;
use nalgebra::Vector2;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);
pub type BodyId = usize;
pub fn next_id() -> BodyId {
    NEXT_ID.fetch_add(1, Ordering::Relaxed) as BodyId
}

/// Axis-aligned bounding box for a body
pub struct BoundingBox<T> {
    pub min: Vector2<T>,
    pub max: Vector2<T>,
}

impl<T: FloatingPointNumber> BoundingBox<T> {
    /// Checks if this bounding box intersects with another bounding box.
    pub fn intersects(&self, other: &BoundingBox<T>) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    /// Creates a bounding box with all values set to zero.
    pub fn zeros() -> Self {
        Self {
            min: Vector2::zeros(),
            max: Vector2::zeros(),
        }
    }

    /// Calculates the center point of the bounding box.
    pub fn center(&self) -> Vector2<T> {
        (self.min + self.max) / T::from(2.0)
    }
}

/// Represents an affine transformation that can be applied to a body, including translation, scaling, and rotation.
#[derive(Clone, Copy)]
pub struct Transformation<T> {
    pub offset: Vector2<T>,
    pub scale: T,
    pub rotation_radians: T,
}

impl<T: FloatingPointNumber> Transformation<T> {
    /// Creates an identity transformation with no translation, unit scale, and no rotation.
    pub fn identity() -> Self {
        Self {
            offset: Vector2::zeros(),
            scale: T::one(),
            rotation_radians: T::zero(),
        }
    }
}

/// Represents a physical body in the simulation, consisting of particles and constraints.
/// Bodies are hierarchical, meaning they can have child bodies with their own particles and constraints.
/// The constraints are categorized into three types:
/// intrinsic constraints: constraints that only  involve body itself
/// extrinsic constraints: constraints are constraints between body and children
/// collision constraint: constraints between children
///
/// Each step external forces are applied to the body and its children, then constraints are solved in the following order:
/// 1. Intrinsic constraints of the body
/// 2. Extrinsic constraints between the body and its children
/// 3. Collision constraints between the body's children
///
/// After all constraints are solved, the body's particles are updated based on their velocities and the time step.
pub struct Body<T = f32> {
    /// Unique identifier for the body, used for referencing in constraints and other operations.
    pub id: BodyId,
    /// Particles that make up the body, representing its physical structure and properties. These need to be in CCW order.
    pub particles: Vec<Particle<T>>,
    /// Intrinsic constraints that maintain the internal structure of the body, such as distance constraints between its particles.
    pub constraints: Vec<Box<dyn IntrinsicConstraint<T>>>,
    pub children: Vec<Body<T>>,
    /// Extrinsic constraints that define how the body interacts with its children, such as attachment constraints or wall constraints.
    pub children_constraints: Vec<ExtrinsicConstraintType<T>>,
    /// Collision constraint that defines how the body's children interact with each other.
    pub collision_constraint: Option<CollisionConstraint<T>>,
}

impl<T: Clone> Clone for Body<T> {
    /// Custom clone implementation to ensure that child bodies get new unique IDs and that local extrinsic constraints are properly remapped to the new child IDs.
    /// This means the body behaves identical to the old one, but has a different id in the system.
    fn clone(&self) -> Self {
        let children: Vec<Body<T>> = self.children.iter().map(Clone::clone).collect();
        let child_id_map: HashMap<BodyId, BodyId> = self
            .children
            .iter()
            .zip(children.iter())
            .map(|(old_child, new_child)| (old_child.id, new_child.id))
            .collect();

        let mut children_constraints = self.children_constraints.clone();
        for constraint in children_constraints.iter_mut() {
            if let ExtrinsicConstraintType::Local(local) = constraint {
                local.remap_body_ids(&child_id_map);
            }
        }

        Self {
            id: next_id(),
            particles: self.particles.clone(),
            constraints: self.constraints.clone(),
            children,
            children_constraints,
            collision_constraint: self.collision_constraint.clone(),
        }
    }
}

impl<T: FloatingPointNumber> Body<T> {
    /// Calculates the centroid of the body based on the average position of its particles.
    pub fn centroid(&self) -> Vector2<T> {
        let n = T::from(self.particles.len() as f32);
        self.particles
            .iter()
            .fold(Vector2::zeros(), |acc, p| acc + p.position)
            / n
    }
    /// Applies a uniform movement to the body by offsetting all particles by the same amount.
    /// This is useful for moving the entire body without affecting its internal structure, such as when
    /// offset_now is an offset applied to the body in the current frame, and offset_last_frame is the offset applied in the previous frame,
    /// to maintain consistent velocity when moving a body.
    pub fn set_uniform_movement(&mut self, offset_now: Vector2<T>, offset_last_frame: Vector2<T>) {
        for particle in self.particles.iter_mut() {
            particle.pre_integration_position = particle.position + offset_last_frame;
            particle.position += offset_now;
        }
        for child in self.children.iter_mut() {
            child.set_uniform_movement(offset_now, offset_last_frame);
        }
    }

    /// Calculates the axis-aligned bounding box that contains all particles of the body.
    pub fn get_bounding_box(&self) -> BoundingBox<T> {
        if let Some((first, rest)) = self.particles.split_first() {
            let (min, max) = rest
                .iter()
                .fold((first.position, first.position), |(min_acc, max_acc), p| {
                    (min_acc.inf(&p.position), max_acc.sup(&p.position))
                });
            BoundingBox { min, max }
        } else {
            BoundingBox::zeros()
        }
    }

    /// Determines if a given point is inside the polygon formed by the body's particles.
    pub fn point_in_polygon(&self, point: Vector2<T>) -> bool {
        if self.particles.len() < 3 {
            return false;
        }

        let mut inside = false;
        for (&Particle { position: a, .. }, &Particle { position: b, .. }) in
            self.particles.iter().circular_tuple_windows()
        {
            let y_intersection = (a.y > point.y) != (b.y > point.y);
            if !y_intersection {
                continue;
            }

            let dy = b.y - a.y;
            if dy.abs() <= T::zero() {
                continue;
            }

            let x_intersection = a.x + (point.y - a.y) * (b.x - a.x) / dy;
            if point.x < x_intersection {
                inside = !inside;
            }
        }
        inside
    }

    /// Applies an affine transformation to the body, including translation, scaling, and rotation.
    pub fn transform(&mut self, transformation: Transformation<T>) {
        self.apply_transformation_recursive(transformation, None);
        self.transform_constraints_recursive(transformation);
    }

    fn apply_transformation_recursive(
        &mut self,
        transformation: Transformation<T>,
        rotation_center: Option<Vector2<T>>,
    ) {
        let cos_theta = transformation.rotation_radians.cos();
        let sin_theta = transformation.rotation_radians.sin();
        let rot_mat = nalgebra::Matrix2::new(cos_theta, -sin_theta, sin_theta, cos_theta);
        let centroid = rotation_center.unwrap_or_else(|| self.get_bounding_box().center());

        let apply_transform = |v: Vector2<T>| {
            let centered = v - centroid;
            let rotated = rot_mat * centered + centroid;
            rotated * transformation.scale + transformation.offset
        };

        for particle in self.particles.iter_mut() {
            particle.position = apply_transform(particle.position);
            particle.pre_integration_position = apply_transform(particle.pre_integration_position);
        }

        for child in self.children.iter_mut() {
            child.apply_transformation_recursive(transformation, Some(centroid));
        }
    }

    fn transform_constraints_recursive(&mut self, transformation: Transformation<T>) {
        for constraint in self.constraints.iter_mut() {
            constraint.transform_params(transformation);
        }

        for child_constraint in self.children_constraints.iter_mut() {
            if let ExtrinsicConstraintType::Local(local) = child_constraint {
                local.transform_params(transformation);
            }
        }

        for child in self.children.iter_mut() {
            child.transform_constraints_recursive(transformation);
        }
    }
    fn update_positions_recursively(&mut self, dt: T, solver_settings: &SolverSettings) {
        for particle in self.particles.iter_mut() {
            particle.post_integration_update(dt, solver_settings);
        }

        for child in self.children.iter_mut() {
            child.update_positions_recursively(dt, solver_settings);
        }
    }

    fn apply_forces_recursively<F>(&mut self, forces: &[F], dt: T)
    where
        F: Force<T>,
    {
        for particle in self.particles.iter_mut().filter(|p| !p.pinned) {
            let force = forces
                .iter()
                .fold(nalgebra::Vector2::zeros(), |acc, force| {
                    acc + force.apply(particle)
                });
            particle.apply_force(&force, dt);
        }
        for child in self.children.iter_mut() {
            child.apply_forces_recursively(forces, dt);
        }
    }

    fn solve_constraints_recursivly(&mut self, dt: T, solver_settings: &SolverSettings) {
        // Solve constraints to maintain this body's internal structure.
        for constraint in &self.constraints {
            constraint.solve(&mut self.particles, dt, solver_settings);
        }

        // Solve constraints between this body and its direct children.
        for constraint in &self.children_constraints {
            match constraint {
                ExtrinsicConstraintType::Global(c) => {
                    c.solve(&mut self.children, &self.particles, dt, solver_settings)
                }
                ExtrinsicConstraintType::Local(c) => {
                    let id = c.get_id();
                    if let Some(child) = self.children.iter_mut().find(|child| child.id == id) {
                        c.solve(child, &self.particles, dt, solver_settings);
                    } else {
                        eprintln!(
                            "Child with id {} not found for local extrinsic constraint",
                            id
                        );
                    }
                }
            }
        }

        for child in self.children.iter_mut() {
            child.solve_constraints_recursivly(dt, solver_settings);
        }

        if let Some(collision_constraint) = &self.collision_constraint {
            let bounding_boxes: Vec<BoundingBox<T>> =
                self.children.iter().map(|c| c.get_bounding_box()).collect();
            for [a_idx, b_idx] in (0..self.children.len()).array_combinations() {
                if !bounding_boxes[a_idx].intersects(&bounding_boxes[b_idx]) {
                    continue;
                }
                let (left, right) = self.children.split_at_mut(b_idx);
                let child_a = &mut left[a_idx];
                let child_b = &mut right[0];
                collision_constraint.solve(child_a, child_b, dt, solver_settings);
            }
        }
    }

    /// Performs a simulation step for the body, applying forces, solving constraints, and updating particle positions.
    pub fn perform_step<F>(&mut self, forces: &[F], dt: T, solver_settings: &SolverSettings)
    where
        F: Force<T>,
    {
        //calculate how external forces would affect the body
        self.apply_forces_recursively(forces, dt);

        //solve constraints of the body and between it and its chidlren
        for _ in 0..solver_settings.constraint_iterations {
            self.solve_constraints_recursivly(dt, solver_settings);
        }
        //update velocities after all forces and constraints are processed
        self.update_positions_recursively(dt, solver_settings);
    }
}

impl<T> Body<T> {
    /// Creates an empty body with a unique ID and no particles, constraints, or children.
    pub fn empty() -> Self {
        let id = next_id();
        Self {
            id,
            particles: Vec::new(),
            constraints: Vec::new(),
            children: Vec::new(),
            children_constraints: Vec::new(),
            collision_constraint: None,
        }
    }

    /// Finds a child body by its unique ID, returning a reference to it if found.
    pub fn find_child_by_id(&self, id: BodyId) -> Option<&Body<T>> {
        if self.id == id {
            return Some(self);
        }

        for child in &self.children {
            if let Some(found) = child.find_child_by_id(id) {
                return Some(found);
            }
        }
        None
    }

    /// Finds a mutable reference to a child body by its unique ID, returning it if found.
    pub fn find_child_by_id_mut(&mut self, id: BodyId) -> Option<&mut Body<T>> {
        if self.id == id {
            return Some(self);
        }

        for child in &mut self.children {
            if let Some(found) = child.find_child_by_id_mut(id) {
                return Some(found);
            }
        }
        None
    }

    /// Sets the pinned state of all particles in the body and its children,
    /// effectively fixing them in place if pinned is true, or allowing them to move if pinned is false.
    pub fn set_pinned(&mut self, pinned: bool) {
        for particle in self.particles.iter_mut() {
            particle.pinned = pinned;
        }
        for child in self.children.iter_mut() {
            child.set_pinned(pinned);
        }
    }
}
