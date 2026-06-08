use crate::Particle;
use nalgebra::Vector2;

pub trait IntrinsicContraint<T = f32> {
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

impl<T> IntrinsicContraint<T> for DistanceConstraint<T>
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

        particles[self.idx_left].apply_position_correction(&correction_vector);
        particles[self.idx_right].apply_position_correction(&-correction_vector);
    }
}

pub struct AreaConstraint<T> {
    pub idxs: Vec<usize>,
    pub rest_area: T,
    pub stiffness: T,
}

impl<T> AreaConstraint<T>
where
    T: nalgebra::RealField + Copy,
{
    pub fn new(idxs: Vec<usize>, particles: &Vec<Particle<T>>, stiffness: T) -> Self {
        let rest_area = Self::calculate_area(&idxs, particles);
        Self {
            idxs,
            rest_area,
            stiffness,
        }
    }

    fn calculate_area(idxs: &Vec<usize>, particles: &Vec<Particle<T>>) -> T {
        let mut area = T::zero();
        for i in 0..idxs.len() {
            let current = &particles[idxs[i]];
            let next = &particles[idxs[(i + 1) % idxs.len()]];
            //det form of trapazoidal rule ad-bc
            area += current.position.x * next.position.y - next.position.x * current.position.y;
        }
        area.abs() * T::from_f32(0.5).unwrap()
    }
}

impl<T> IntrinsicContraint<T> for AreaConstraint<T>
where
    T: nalgebra::RealField + Copy,
{
    fn solve(&self, particles: &mut Vec<Particle<T>>, dt: &T) {
        let current_area = Self::calculate_area(&self.idxs, particles);
        let area_error = self.rest_area - current_area;
        let correction_magnitude = *dt * self.stiffness * area_error / self.rest_area;
        let correction_vector = Vector2::new(-correction_magnitude, correction_magnitude);
        for idx in &self.idxs {
            if !particles[*idx].pinned {
                particles[*idx].apply_position_correction(&correction_vector);
            }
        }
    }
}
