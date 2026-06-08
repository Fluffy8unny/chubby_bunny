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

    pub fn perform_step<F>(&mut self, forces: &Vec<F>, dt: T)
    where
        F: Force<T>,
        T: nalgebra::RealField + Copy,
    {
        let initial_positions: Vec<_> = self.particles.iter().map(|p| p.position).collect();

        // Update particle positions based on their velocities and apply external forces
        for particle in self.particles.iter_mut().filter(|p| !p.pinned) {
            let force = forces
                .iter()
                .fold(nalgebra::Vector2::zeros(), |acc, force| {
                    acc + force.apply(particle)
                });
            let acceleration = force / particle.mass;
            let velocity = particle.velocity + acceleration * dt;
            particle.apply_position_correction(&(velocity * dt));
        }
        for i in 0..10 {
            // Solve extrinsic constraints between this body and its children
            for constraint in &self.children_constraints {
                constraint.solve(&mut self.children, &self.particles);
            }
            // Solve constraints to maintain the structure of the body
            for constraint in &self.constraints {
                constraint.solve(&mut self.particles);
            }
        }
        //update velocities after all forces and constraints are processed
        if dt > T::zero() {
            for (particle, pre_pos) in self.particles.iter_mut().zip(initial_positions.iter()) {
                if !particle.pinned {
                    particle.velocity =
                        (particle.position - *pre_pos) * ((T::one() - particle.friction) / dt);
                }
            }
        }

        for child in &mut self.children {
            child.perform_step(forces, dt);
        }
    }
}
