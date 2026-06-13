use crate::collision_constraint::CollisionConstraint;
use crate::{
    ExtrinsicConstraintType, FloatingPointNumber, Force, IntrinsicContraint, Particle,
    SolverSettings,
};
use itertools::Itertools;
use nalgebra::Vector2;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);
pub type BodyId = usize;
pub fn next_id() -> BodyId {
    NEXT_ID.fetch_add(1, Ordering::Relaxed) as BodyId
}
pub struct Body<T = f32> {
    pub id: BodyId,
    pub particles: Vec<Particle<T>>,
    pub constraints: Vec<Box<dyn IntrinsicContraint<T>>>,
    pub children: Vec<Body<T>>,
    pub children_constraints: Vec<ExtrinsicConstraintType<T>>,
    pub collision_constraint: Option<CollisionConstraint<T>>,
}
pub struct BoundingBox<T> {
    pub min: Vector2<T>,
    pub max: Vector2<T>,
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
            self.move_uniform(offset);
        } else {
            for child in self.children.iter_mut() {
                child.move_child_by_id(id, offset);
            }
        }
    }

    pub fn move_uniform(&mut self, offset: Vector2<T>)
    where
        T: FloatingPointNumber,
    {
        for particle in self.particles.iter_mut() {
            particle.pre_integration_position = particle.position;
            particle.position += offset;
        }
        for child in self.children.iter_mut() {
            child.move_uniform(offset);
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
