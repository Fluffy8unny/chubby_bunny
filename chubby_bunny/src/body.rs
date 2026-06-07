use crate::Constraint;
use crate::Force;
use crate::Particle;
pub struct Body<T = f32> {
    pub particles: Vec<Particle<T>>,
    pub constraints: Vec<Box<dyn Constraint<T>>>,
}

impl<T> Body<T> {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            constraints: Vec::new(),
        }
    }

    pub fn perform_step<F>(&mut self, force: F, dt: T)
    where
        F: Force<T>,
        T: nalgebra::RealField + Copy,
    {
        // Update particle positions based on their velocities and apply external forces
        for particle in self.particles.iter_mut().filter(|p| !p.pinned) {
            let f = force.apply(particle);
            particle.update(&f, &dt);
        }

        // Solve constraints to maintain the structure of the body
        for constraint in &self.constraints {
            constraint.solve(&mut self.particles, &dt);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::force::no_force;
    use nalgebra::Vector2;
    #[test]
    pub fn two_particle_body_test() {
        let mut body = Body::new();
        body.particles.push(Particle {
            position: Vector2::new(0.0, 0.0),
            velocity: Vector2::new(0.0, 0.0),
            friction: 0.1,
            mass: 1.0,
            pinned: false,
        });
        body.particles.push(Particle {
            position: Vector2::new(1.0, 0.0),
            velocity: Vector2::new(0.0, 0.0),
            friction: 0.1,
            mass: 1.0,
            pinned: false,
        });
        body.constraints
            .push(Box::new(crate::constraint::DistanceConstraint::new(
                0,
                1,
                &body.particles,
                1.0,
            )));
        //no forces, so the particles should not move and the distance constraint should keep them at the same distance
        for _ in 1..10 {
            body.perform_step(no_force(), 0.1);
            print!(
                "particle 0 position: {:?}, velocity: {:?}\n",
                body.particles[0].position, body.particles[0].velocity
            );
            print!(
                "particle 1 position: {:?}, velocity: {:?}\n\n",
                body.particles[1].position, body.particles[1].velocity
            );
            assert_eq!(body.particles[0].position, nalgebra::Vector2::new(0.0, 0.0));
            assert_eq!(body.particles[1].position, nalgebra::Vector2::new(1.0, 0.0));
        }
    }
}
