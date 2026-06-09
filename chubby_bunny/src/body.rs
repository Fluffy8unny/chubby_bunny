use crate::Force;
use crate::GlobalExtrinsicConstraint;
use crate::IntrinsicContraint;
use crate::LocalExtrinsicConstraint;
use crate::Particle;
use crate::SolverSettings;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);
pub type BodyId = usize;
pub fn next_id() -> BodyId {
    NEXT_ID.fetch_add(1, Ordering::Relaxed) as BodyId
}
pub enum ExtrinsicConstraintType<T> {
    Global(Box<dyn GlobalExtrinsicConstraint<T>>),
    Local(Box<dyn LocalExtrinsicConstraint<T>>),
}
pub struct Body<T = f32> {
    pub id: BodyId,
    pub particles: Vec<Particle<T>>,
    pub constraints: Vec<Box<dyn IntrinsicContraint<T>>>,
    pub children: HashMap<BodyId, Body<T>>,
    pub children_constraints: Vec<ExtrinsicConstraintType<T>>,
}

impl<T> Body<T> {
    pub fn empty() -> Self {
        let id = next_id();
        Self {
            id,
            particles: Vec::new(),
            constraints: Vec::new(),
            children: HashMap::new(),
            children_constraints: Vec::new(),
        }
    }

    fn update_positions_recursively(&mut self, dt: T, solver_settings: &SolverSettings)
    where
        T: nalgebra::RealField + Copy + From<f32>,
    {
        for particle in self.particles.iter_mut() {
            particle.post_integration_update(dt, solver_settings);
        }

        for child in self.children.values_mut() {
            child.update_positions_recursively(dt, solver_settings);
        }
    }

    fn apply_forces_recursively<F>(&mut self, forces: &Vec<F>, dt: T)
    where
        F: Force<T>,
        T: nalgebra::RealField + Copy,
    {
        for particle in self.particles.iter_mut().filter(|p| !p.pinned) {
            let force = forces
                .iter()
                .fold(nalgebra::Vector2::zeros(), |acc, force| {
                    acc + force.apply(particle)
                });
            particle.apply_force(&force, dt);
        }
        for child in self.children.values_mut() {
            child.apply_forces_recursively(forces, dt);
        }
    }

    fn solve_constraints_recursivly(&mut self, dt: T, solver_settings: &SolverSettings)
    where
        T: nalgebra::RealField + Copy + From<f32>,
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
                    if let Some(child) = self.children.get_mut(&id) {
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

        for child in self.children.values_mut() {
            child.solve_constraints_recursivly(dt, solver_settings);
        }
        //todo: not implemented yet
        //self.solve_children_collisions(dt);
    }

    pub fn perform_step<F>(&mut self, forces: &Vec<F>, dt: T, solver_settings: &SolverSettings)
    where
        F: Force<T>,
        T: nalgebra::RealField + Copy + From<f32>,
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
