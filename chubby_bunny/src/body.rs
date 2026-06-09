use crate::ExtrinsicConstraint;
use crate::Force;
use crate::IntrinsicContraint;
use crate::Particle;
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
    pub children_constraints: Vec<Box<dyn ExtrinsicConstraint<T>>>,
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
        }
    }

    fn update_positions_recursively(&mut self, dt: T)
    where
        T: nalgebra::RealField + Copy,
    {
        for particle in self.particles.iter_mut() {
            particle.post_integration_update(dt);
        }
        for child in &mut self.children {
            child.update_positions_recursively(dt);
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
        for child in &mut self.children {
            child.apply_forces_recursively(forces, dt);
        }
    }

    fn solve_constraints_recursivly(&mut self, dt: T, num_iterations: usize)
    where
        T: nalgebra::RealField + Copy + From<f32>,
    {
        for _ in 0..num_iterations {
            let t_per_itt = dt / T::from(num_iterations as f32);

        // Solve constraints between this body and its direct children.
        for constraint in &self.children_constraints {
            constraint.solve(&mut self.children, &self.particles, &t_per_itt);
        }
        
        // Solve constraints to maintain this body's internal structure.
        for constraint in &self.constraints {
            constraint.solve(&mut self.particles, &t_per_itt);
        }

        for child in &mut self.children {
            child.solve_constraints_recursivly(t_per_itt, num_iterations);
        }

    }
        //todo: not implemented yet
        //self.solve_children_collisions(dt);
    }

    pub fn perform_step<F>(&mut self, forces: &Vec<F>, dt: T)
    where
        F: Force<T>,
        T: nalgebra::RealField + Copy + From<f32>,
    {
        //calculate how external forces would affect the body
        self.apply_forces_recursively(forces, dt);

        //solve constraints of the body and between it and its chidlren
        self.solve_constraints_recursivly(dt, 10_usize);
        
        //update velocities after all forces and constraints are processed
        self.update_positions_recursively(dt);

    }
}
