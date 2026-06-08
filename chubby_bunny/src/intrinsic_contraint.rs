use crate::constraint_utils::constraint_alpha_with_reference_dt;
use crate::Particle;
use nalgebra::Vector2;

pub trait IntrinsicContraint<T = f32> {
    fn solve(&self, particles: &mut Vec<Particle<T>>, dt: &T);
}

pub struct DistanceConstraint<T> {
    pub idx_left: usize,
    pub idx_right: usize,
    pub target_distance: T,
    pub stiffness: T,
    pub fps: T,
}

impl<T> DistanceConstraint<T>
where
    T: nalgebra::RealField + Copy + From<f32>,
{
    pub fn new(
        idx_left: usize,
        idx_right: usize,
        particles: &Vec<Particle<T>>,
        stiffness: T,
        fps: T,
    ) -> Self {
        let target_distance = (particles[idx_right].position - particles[idx_left].position).norm();
        Self {
            idx_left,
            idx_right,
            target_distance,
            stiffness,
            fps,
        }
    }
}

impl<T> IntrinsicContraint<T> for DistanceConstraint<T>
where
    T: nalgebra::RealField + Copy + From<f32>,
{
    fn solve(&self, particles: &mut Vec<Particle<T>>, dt: &T) {
        let line_between = particles[self.idx_right].position - particles[self.idx_left].position;
        let point_distance = line_between.norm();
        if point_distance <= T::zero() {
            return;
        }
        let move_direction = line_between / point_distance;
        let alpha = constraint_alpha_with_reference_dt(self.stiffness, *dt, self.fps);

        let correction_magnitude = alpha * (self.target_distance - point_distance) / T::from(2.0);
        let correction_vector = move_direction * correction_magnitude;

        particles[self.idx_left].apply_position_correction(&-correction_vector);
        particles[self.idx_right].apply_position_correction(&correction_vector);
    }
}

pub struct AreaConstraint<T> {
    pub idxs: Vec<usize>,
    pub rest_area: T,
    pub stiffness: T,
    pub fps: T,
}

impl<T> AreaConstraint<T>
where
    T: nalgebra::RealField + Copy + From<f32>,
{
    pub fn new(idxs: Vec<usize>, particles: &Vec<Particle<T>>, stiffness: T, fps: T) -> Self {
        let rest_area = Self::calculate_area(&idxs, particles);
        Self {
            idxs,
            rest_area,
            stiffness,
            fps: fps,
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
        area.abs() / T::from(2.0)
    }
}

impl<T> IntrinsicContraint<T> for AreaConstraint<T>
where
    T: nalgebra::RealField + Copy + From<f32>,
{
    fn solve(&self, particles: &mut Vec<Particle<T>>, dt: &T) {
        let current_area = Self::calculate_area(&self.idxs, particles);
        if current_area <= T::zero() {
            return;
        }

        let n = T::from(self.idxs.len() as f32);
        let centroid = self
            .idxs
            .iter()
            .fold(Vector2::zeros(), |acc, &i| acc + particles[i].position)
            / n;

        // area scales as the square of linear scale, so linear scale factor is sqrt(rest/current)
        let scale_correction = (self.rest_area / current_area).sqrt() - T::one();
        let alpha = constraint_alpha_with_reference_dt(self.stiffness, *dt, self.fps);

        for idx in &self.idxs {
            let offset = particles[*idx].position - centroid;
            particles[*idx].apply_position_correction(&(offset * scale_correction * alpha));
        }
    }
}
