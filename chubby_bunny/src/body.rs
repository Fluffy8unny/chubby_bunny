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
            let f = forces
                .iter()
                .fold(nalgebra::Vector2::zeros(), |acc, force| {
                    acc + force.apply(particle)
                });
            particle.physics_update(&f, &dt);
        }

        // Solve constraints to maintain the structure of the body
        for constraint in &self.constraints {
            constraint.solve(&mut self.particles, &dt);
        }

        // Integrate child bodies once per frame. The iterative loop below is for projection only.
        for child in &mut self.children {
            child.perform_step(forces, dt);
        }

        // Solve extrinsic constraints between this body and its children
        for constraint in &self.children_constraints {
            constraint.solve(&mut self.children, &self.particles, &dt);
        }

        // We fucked with physics before, we need to update the velocities based on the position changes after constraint projection
        //like all great physists say: when stuff doesn't work out just add a correction term
        if dt > T::zero() {
            for (particle, pre_pos) in self.particles.iter_mut().zip(initial_positions.iter()) {
                if !particle.pinned {
                    particle.velocity = (particle.position - *pre_pos) / dt;
                }
            }
        }
    }
}
