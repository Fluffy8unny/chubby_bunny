use crate::constraint_utils::constraint_alpha_with_reference_dt;
use crate::Body;
use crate::Particle;
use nalgebra::Vector2;

pub trait ExtrinsicConstraint<T = f32> {
    fn solve(&self, bodies: &mut Vec<Body<T>>, parent_particles: &[Particle<T>], dt: &T);
}

pub struct WallConstraint<T> {
    pub idx_body: usize,
    pub parent_point_idx_origin: usize,
    pub parent_point_idx_end: usize,
    pub stiffness: T,
    pub fps: T,
}

impl<T> ExtrinsicConstraint<T> for WallConstraint<T>
where
    T: nalgebra::RealField + Copy + From<f32>,
{
    fn solve(&self, bodies: &mut Vec<Body<T>>, parent_particles: &[Particle<T>], dt: &T) {
        let alpha = constraint_alpha_with_reference_dt(self.stiffness, *dt, self.fps);
        let body = &mut bodies[self.idx_body];
        for particle in body.particles.iter_mut().filter(|p| !p.pinned) {
            //calculate line based on parent points
            let line_origin = parent_particles[self.parent_point_idx_origin].position;
            let line_end = parent_particles[self.parent_point_idx_end].position;
            let line_direction = line_end - line_origin;
            let line_normal = Vector2::new(-line_direction.y, line_direction.x).normalize();

            let to_particle = particle.position - line_origin;
            let distance = to_particle.dot(&line_normal);
            if distance < T::zero() {
                let correction_vector = line_normal * (-distance * alpha);
                particle.apply_position_correction(&correction_vector);

                let reflection_correction_term = line_normal * particle.velocity.dot(&line_normal);
                let reflected_velocity: Vector2<T> =
                    particle.velocity - reflection_correction_term * alpha / T::from(2.0);
                particle.velocity = reflected_velocity * alpha;
            }
        }
    }
}

pub struct AttachmentConstraint<T> {
    pub idx_body: usize,
    pub point_idxs_parent: Vec<usize>,
    pub point_idxs_child: Vec<usize>,
    pub stiffness: T,
    pub fps: T,
}
impl<T> AttachmentConstraint<T> {
    pub fn new(
        idx_body: usize,
        point_idxs_parent: Vec<usize>,
        point_idxs_child: Vec<usize>,
        stiffness: T,
        fps: T,
    ) -> Self {
        assert_eq!(
            point_idxs_parent.len(),
            point_idxs_child.len(),
            "Parent and child point index lists must be of the same length"
        );
        Self {
            idx_body,
            point_idxs_parent,
            point_idxs_child,
            stiffness,
            fps,
        }
    }
}
impl<T> ExtrinsicConstraint<T> for AttachmentConstraint<T>
where
    T: nalgebra::RealField + Copy + From<f32>,
{
    fn solve(&self, bodies: &mut Vec<Body<T>>, parent_particles: &[Particle<T>], dt: &T) {
        let alpha = constraint_alpha_with_reference_dt(self.stiffness, *dt, self.fps);
        let body = &mut bodies[self.idx_body];
        for (parent_idx, child_idx) in self
            .point_idxs_parent
            .iter()
            .zip(self.point_idxs_child.iter())
        {
            let parent_particle = &parent_particles[*parent_idx];
            let child_particle = &body.particles[*child_idx];

            let correction_vector = (parent_particle.position - child_particle.position) * alpha;
            body.particles[*child_idx].apply_position_correction(&correction_vector);
        }
    }
}
