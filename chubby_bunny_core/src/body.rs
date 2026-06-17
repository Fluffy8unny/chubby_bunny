use crate::collision_constraint::CollisionConstraint;
use crate::{
    ExtrinsicConstraintType, FloatingPointNumber, Force, IntrinsicConstraint, Particle,
    SolverSettings,
};
use dyn_clone::clone_box;
use itertools::Itertools;
use nalgebra::Vector2;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);
pub type BodyId = usize;
pub fn next_id() -> BodyId {
    NEXT_ID.fetch_add(1, Ordering::Relaxed) as BodyId
}
#[derive(Clone)]
pub struct Body<T = f32> {
    pub id: BodyId,
    pub particles: Vec<Particle<T>>,
    pub constraints: Vec<Rc<dyn IntrinsicConstraint<T>>>,
    pub children: Vec<Body<T>>,
    pub children_constraints: Vec<ExtrinsicConstraintType<T>>,
    pub collision_constraint: Option<CollisionConstraint<T>>,
}
pub struct BoundingBox<T> {
    pub min: Vector2<T>,
    pub max: Vector2<T>,
}

impl<T: FloatingPointNumber> BoundingBox<T> {
    pub fn intersects(&self, other: &BoundingBox<T>) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }
}

#[derive(Clone, Copy)]
pub struct Transformation<T> {
    pub offset: Vector2<T>,
    pub scale: T,
    pub rotation_radians: T,
}

impl<T: FloatingPointNumber> Transformation<T> {
    pub fn identity() -> Self {
        Self {
            offset: Vector2::zeros(),
            scale: T::one(),
            rotation_radians: T::zero(),
        }
    }
}

impl<T> Body<T> {
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

    pub fn centroid(&self) -> Vector2<T>
    where
        T: FloatingPointNumber,
    {
        let n = T::from(self.particles.len() as f32);
        self.particles
            .iter()
            .fold(Vector2::zeros(), |acc, p| acc + p.position)
            / n
    }

    pub fn get_bounding_box(&self) -> BoundingBox<T>
    where
        T: FloatingPointNumber,
    {
        self.particles.iter().fold(
            BoundingBox {
                min: Vector2::new(T::max_value().unwrap(), T::max_value().unwrap()),
                max: Vector2::new(T::min_value().unwrap(), T::min_value().unwrap()),
            },
            |mut bbox, particle| {
                bbox.min.x = bbox.min.x.min(particle.position.x);
                bbox.min.y = bbox.min.y.min(particle.position.y);
                bbox.max.x = bbox.max.x.max(particle.position.x);
                bbox.max.y = bbox.max.y.max(particle.position.y);
                bbox
            },
        )
    }

    pub fn point_in_polygon(&self, point: Vector2<T>) -> bool
    where
        T: FloatingPointNumber,
    {
        if self.particles.len() < 3 {
            return false;
        }

        let mut inside = false;
        for (a, b) in self.particles.iter().circular_tuple_windows() {
            let a = a.position;
            let b = b.position;

            let intersects = (a.y > point.y) != (b.y > point.y);
            if !intersects {
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

    pub fn pin_child_by_id(&mut self, id: BodyId, pinned: bool) {
        if self.id == id {
            self.set_pinned(pinned);
        } else {
            for child in self.children.iter_mut() {
                child.pin_child_by_id(id, pinned);
            }
        }
    }

    pub fn set_pinned(&mut self, pinned: bool) {
        for particle in self.particles.iter_mut() {
            particle.pinned = pinned;
        }
        for child in self.children.iter_mut() {
            child.set_pinned(pinned);
        }
    }

    pub fn move_child_by_id(&mut self, id: BodyId, offset: Vector2<T>)
    where
        T: FloatingPointNumber,
    {
        if self.id == id {
            self.move_uniform(offset, Vector2::zeros());
        } else {
            for child in self.children.iter_mut() {
                child.move_child_by_id(id, offset);
            }
        }
    }

    pub fn move_uniform(&mut self, offset_now: Vector2<T>, offset_last_frame: Vector2<T>)
    where
        T: FloatingPointNumber,
    {
        for particle in self.particles.iter_mut() {
            particle.pre_integration_position = particle.position + offset_last_frame;
            particle.position += offset_now;
        }
        for child in self.children.iter_mut() {
            child.move_uniform(offset_now, offset_last_frame);
        }
    }

    pub fn duplicate(&self) -> Self
    where
        T: FloatingPointNumber,
    {
        let mut copy = self.clone();
        let mut id_map = HashMap::new();
        copy.reassign_ids_recursive(&mut id_map);
        copy.remap_local_constraints_recursive(&id_map);
        copy
    }

    pub fn duplicate_with_transformation(&self, transformation: Transformation<T>) -> Self
    where
        T: FloatingPointNumber,
    {
        let mut copy = self.duplicate();
        copy.apply_transformation_recursive(transformation, None);
        copy.transform_constraints_recursive(transformation);
        copy
    }

    pub fn duplicate_with_offset(&self, offset: Vector2<T>) -> Self
    where
        T: FloatingPointNumber,
    {
        self.duplicate_with_transformation(Transformation {
            offset,
            scale: T::one(),
            rotation_radians: T::zero(),
        })
    }

    fn apply_transformation_recursive(
        &mut self,
        transformation: Transformation<T>,
        centroid: Option<Vector2<T>>,
    ) where
        T: FloatingPointNumber,
    {
        let cos_theta = transformation.rotation_radians.cos();
        let sin_theta = transformation.rotation_radians.sin();
        let centroid = centroid.unwrap_or_else(|| {
            let bbox = self.get_bounding_box();
            (bbox.min + bbox.max) / T::from(2.0)
        });

        let apply_to_vector = |v: Vector2<T>| {
            let cenrtered = v - centroid;
            let rotated = Vector2::new(
                cenrtered.x * cos_theta - cenrtered.y * sin_theta,
                cenrtered.x * sin_theta + cenrtered.y * cos_theta,
            ) + centroid;
            rotated * transformation.scale + transformation.offset
        };

        for particle in self.particles.iter_mut() {
            particle.position = apply_to_vector(particle.position);
            particle.pre_integration_position = apply_to_vector(particle.pre_integration_position);
        }

        for child in self.children.iter_mut() {
            child.apply_transformation_recursive(transformation, Some(centroid));
        }
    }

    fn transform_constraints_recursive(&mut self, transformation: Transformation<T>)
    where
        T: FloatingPointNumber,
    {
        for constraint in self.constraints.iter_mut() {
            let mut cloned = clone_box(&**constraint);
            cloned.scale_params(transformation.scale);
            cloned.rotate_params(transformation.rotation_radians);
            *constraint = Rc::from(cloned);
        }

        for child_constraint in self.children_constraints.iter_mut() {
            if let ExtrinsicConstraintType::Local(local) = child_constraint {
                local.scale_params(transformation.scale);
                local.rotate_params(transformation.rotation_radians);
            }
        }

        for child in self.children.iter_mut() {
            child.transform_constraints_recursive(transformation);
        }
    }

    fn reassign_ids_recursive(&mut self, id_map: &mut HashMap<BodyId, BodyId>) {
        let old_id = self.id;
        self.id = next_id();
        id_map.insert(old_id, self.id);
        for child in self.children.iter_mut() {
            child.reassign_ids_recursive(id_map);
        }
    }

    fn remap_local_constraints_recursive(&mut self, id_map: &HashMap<BodyId, BodyId>) {
        for constraint in self.children_constraints.iter_mut() {
            if let ExtrinsicConstraintType::Local(local) = constraint {
                local.remap_body_ids(id_map);
            }
        }

        for child in self.children.iter_mut() {
            child.remap_local_constraints_recursive(id_map);
        }
    }

    fn update_positions_recursively(&mut self, dt: T, solver_settings: &SolverSettings)
    where
        T: FloatingPointNumber,
    {
        for particle in self.particles.iter_mut() {
            particle.post_integration_update(dt, solver_settings);
        }

        for child in self.children.iter_mut() {
            child.update_positions_recursively(dt, solver_settings);
        }
    }

    fn apply_forces_recursively<F>(&mut self, forces: &Vec<F>, dt: T)
    where
        F: Force<T>,
        T: FloatingPointNumber,
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

    fn solve_constraints_recursivly(&mut self, dt: T, solver_settings: &SolverSettings)
    where
        T: FloatingPointNumber,
    {
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
            for (a_idx, b_idx) in (0..self.children.len()).tuple_combinations() {
                let (left, right) = self.children.split_at_mut(b_idx);
                let child_a = &mut left[a_idx];
                let child_b = &mut right[0];
                collision_constraint.solve(child_a, child_b, dt, solver_settings);
            }
        }
    }

    pub fn perform_step<F>(&mut self, forces: &Vec<F>, dt: T, solver_settings: &SolverSettings)
    where
        F: Force<T>,
        T: FloatingPointNumber,
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
