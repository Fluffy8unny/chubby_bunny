use crate::{FloatingPointNumber, Particle};
use nalgebra::Vector2;

/// Force that can be applied to particles, such as gravity or user interaction forces.
pub trait Force<T = f32> {
    fn apply(&self, particle: &Particle<T>) -> Vector2<T>;
}

/// Creates a constant force that applies the same force vector to all particles.
/// This can be used to implement gravity by providing a downward force vector.
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

/// Creates a force that applies no force to any particle.
pub fn no_force<T: FloatingPointNumber>() -> impl Force<T> {
    constant_force(Vector2::zeros())
}

/// Enum used to describe how the strength of a point-based force decays with distance. This is used by `point_based_force`.
pub enum ForceDecayType {
    /// The force strength remains constant regardless of distance.
    Constant,
    /// The force strength decreases linearly with distance.
    Linear,
    /// The force strength decreases quadratically with distance.
    Quadratic,
}

/// Creates a force that attracts particles towards a specific target point, with strength that can decay based on distance.
/// This can be used to implement user interaction forces, deflectors or attractors.
pub fn point_based_force<T: FloatingPointNumber>(
    target: Vector2<T>,
    strength: T,
    decay: ForceDecayType,
) -> impl Force<T> {
    struct DistanceBasedForce<T> {
        target: Vector2<T>,
        strength: T,
        decay: ForceDecayType,
    }

    impl<T: FloatingPointNumber> Force<T> for DistanceBasedForce<T> {
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
