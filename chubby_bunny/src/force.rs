use crate::Particle;
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

pub fn no_force<T>() -> impl Force<T>
where
    T: nalgebra::RealField + Copy,
{
    constant_force(Vector2::zeros())
}
