use nalgebra::Vector2;

#[derive(Debug, Clone)]
pub struct Particle<T = f32> {
    pub position: Vector2<T>,
    pub velocity: Vector2<T>,
    pub mass: T,
    pub friction: T,
    pub pinned: bool,
}

impl<T> Particle<T>
where
    T: nalgebra::RealField + Copy,
{
    pub fn apply_position_correction(&mut self, position_correction: &Vector2<T>) {
        if self.pinned {
            return;
        }
        self.position += *position_correction;
    }
}
