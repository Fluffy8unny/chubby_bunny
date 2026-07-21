use crate::SolverSettings;
use nalgebra::Vector2;

/// Represents a point that descirbes the edge of a body, with physical properties such as mass and friction.
/// Particles are the fundamental building blocks of bodies, and constraints operate on the particles of one or more bodies to enforce physical relationships.
#[derive(Debug, Clone)]
pub struct Particle<T = f32> {
    /// Current position of the particle.
    pub position: Vector2<T>,
    /// Position of the particle before the current integration step. This is used to calculate velocity via verlet integration.
    pub pre_integration_position: Vector2<T>,
    /// Current velocity of the particle.
    pub velocity: Vector2<T>,
    /// Mass of the particle, which influences how it responds to forces and constraints.
    pub mass: T,
    /// Friction coefficient of the particle, which influences how it loses velocity over time.
    pub friction: T,
    /// Whether the particle is pinned in place and should not move in response to forces or constraints.
    pub pinned: bool,
}

impl<T> Particle<T>
where
    T: nalgebra::RealField + Copy,
{
    /// Creates a new particle with the specified properties.
    pub fn new(
        position: Vector2<T>,
        velocity: Vector2<T>,
        mass: T,
        friction: T,
        pinned: bool,
    ) -> Self {
        assert!(mass > T::zero(), "Particle mass must be strictly positive");
        Self {
            position,
            pre_integration_position: position,
            velocity,
            mass,
            friction,
            pinned,
        }
    }

    /// Applies a force to the particle, updating its velocity and position accordingly.
    pub fn apply_force(&mut self, force: &Vector2<T>, dt: T) {
        let acceleration = force / self.mass;
        let velocity = self.velocity + acceleration * dt;
        self.apply_position_correction_to_particle(&(velocity * dt));
    }

    /// Applies a position correction to the particle, typically used to resolve constraints.
    /// Pinned particles ignore position corrections to keep them fixed in place.
    #[inline]
    pub fn apply_position_correction_to_particle(&mut self, position_correction: &Vector2<T>) {
        if self.pinned {
            return;
        }
        self.position += *position_correction;
    }

    /// Verlit integration update that should be called after all forces and constraints have been applied,
    /// to update the particle's velocity based on its movement during the integration step.
    pub fn post_integration_update(&mut self, dt: T, solver_settings: &SolverSettings)
    where
        T: nalgebra::RealField + Copy + From<f32>,
    {
        if dt > T::zero() && !self.pinned {
            let decay = T::one() - self.friction * (dt / T::from(solver_settings.reference_dt));
            self.velocity = (self.position - self.pre_integration_position) * decay / dt;
            self.pre_integration_position = self.position;
        }
    }
}
