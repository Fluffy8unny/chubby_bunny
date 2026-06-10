use crate::collision_constraint::CollisionConstraint;
use crate::{ExtrinsicConstraintType, Force, IntrinsicContraint, Number, Particle, SolverSettings};
use itertools::Itertools;
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
    fn update_positions_recursively(&mut self, dt: T, solver_settings: &SolverSettings)
    where
        T: Number,
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
        T: Number,
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
        T: Number,
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
        T: Number,
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
