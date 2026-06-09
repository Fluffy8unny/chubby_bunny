use nalgebra::Vector2;

#[derive(Debug, Clone)]
pub struct Particle<T = f32> {
    pub position: Vector2<T>,
    pub pre_integration_position: Vector2<T>,
    pub velocity: Vector2<T>,
    pub mass: T,
    pub friction: T,
    pub pinned: bool,
}

impl<T> Particle<T>
where
    T: nalgebra::RealField + Copy,
{
    pub fn new(
        position: Vector2<T>,
        velocity: Vector2<T>,
        mass: T,
        friction: T,
        pinned: bool,
    ) -> Self {
        Self {
            position,
            pre_integration_position: position,
            velocity,
            mass,
            friction,
            pinned,
        }
    }

    pub fn apply_force(&mut self, force: &Vector2<T>, dt: T) {
        let acceleration = force / self.mass;
        let velocity = self.velocity + acceleration * dt;
        self.apply_position_correction(&(velocity * dt));
    }

    pub fn apply_position_correction(&mut self, position_correction: &Vector2<T>) {
        if self.pinned {
            return;
        }
        self.position += *position_correction;
    }

    pub fn post_integration_update(&mut self, dt: T) {
        if dt > T::zero() && !self.pinned {
            self.velocity =
                (self.position - self.pre_integration_position) * (T::one() - self.friction) / dt;
            self.pre_integration_position = self.position;
        }
    }
}
