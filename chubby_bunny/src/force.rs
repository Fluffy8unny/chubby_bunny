use crate::{Number, Particle};
use nalgebra::Vector2;

pub trait Force<T = f32> {
    fn apply(&self, particle: &Particle<T>) -> Vector2<T>;
}

pub fn constant_force<T>(f: Vector2<T>) -> impl Force<T>
where
    T: Copy,
{
    struct ConstantForce<T> {
        f: Vector2<T>,
    }

    impl<T> Force<T> for ConstantForce<T>
    where
        T: Copy,
    {
        fn apply(&self, _: &Particle<T>) -> Vector2<T> {
            self.f
        }
    }
    ConstantForce { f }
}

pub fn no_force<T: Number>() -> impl Force<T> {
    constant_force(Vector2::zeros())
}

pub enum ForceDecayType {
    Constant,
    Linear,
    Quadratic,
}

pub fn point_based_force<T: Number>(
    target: Vector2<T>,
    strength: T,
    decay: ForceDecayType,
) -> impl Force<T> {
    struct DistanceBasedForce<T> {
        target: Vector2<T>,
        strength: T,
        decay: ForceDecayType,
    }

    impl<T: Number> Force<T> for DistanceBasedForce<T> {
        fn apply(&self, particle: &Particle<T>) -> Vector2<T> {
            let direction = self.target - particle.position;
            let decayed_strength = match self.decay {
                ForceDecayType::Constant => self.strength,
                ForceDecayType::Linear => self.strength / direction.norm(),
                ForceDecayType::Quadratic => self.strength / direction.norm_squared(),
            };
            direction.normalize() * decayed_strength
        }
    }
    DistanceBasedForce {
        target,
        strength,
        decay,
    }
}
