use crate::Particle;

pub trait Constraint<T = f32> {
    fn solve(&self, particles: &mut Vec<Particle<T>>, dt: &T);
}

pub struct DistanceConstraint<T> {
    pub idx_left: usize,
    pub idx_right: usize,
    pub rest_length: T,
    pub stiffness: T,
}

impl<T> DistanceConstraint<T>
where
    T: nalgebra::RealField + Copy,
{
    pub fn new(
        idx_left: usize,
        idx_right: usize,
        particles: &Vec<Particle<T>>,
        stiffness: T,
    ) -> Self {
        let rest_length = (particles[idx_right].position - particles[idx_left].position).norm();
        Self {
            idx_left,
            idx_right,
            rest_length,
            stiffness,
        }
    }
}

impl<T> Constraint<T> for DistanceConstraint<T>
where
    T: nalgebra::RealField + Copy,
{
    fn solve(&self, particles: &mut Vec<Particle<T>>, dt: &T) {
        let point_distance = particles[self.idx_right].position - particles[self.idx_left].position;
        let delta_length = point_distance.norm();
        let move_direction = point_distance / delta_length;

        let relative_correction = (self.rest_length - delta_length) / self.rest_length;
        let correction_magnitude = *dt * self.stiffness * relative_correction / delta_length;
        let correction_vector = move_direction * correction_magnitude;
        if !particles[self.idx_left].pinned {
            particles[self.idx_left].position -= correction_vector;
        }
        if !particles[self.idx_right].pinned {
            particles[self.idx_right].position += correction_vector;
        }
    }
}
