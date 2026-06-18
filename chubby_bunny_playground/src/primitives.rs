use chubby_bunny_core::particle::Particle;
use chubby_bunny_core::FloatingPointNumber;
use chubby_bunny_core::{AreaConstraint, Body, DistanceConstraint};
use nalgebra::Vector2;

pub fn create_polygon<T: FloatingPointNumber>(
    center: Vector2<T>,
    radius: T,
    num_sides: usize,
    stiffness_distance: T,
    stiffness_shear: T,
    stiffness_area: T,
    friction: T,
) -> Body<T> {
    let mut polygon = Body::empty();
    for i in 0..num_sides {
        let angle = (i as f32 / num_sides as f32) * std::f32::consts::TAU;
        let position = center + Vector2::new(T::from(angle.cos()), T::from(angle.sin())) * radius;
        polygon.particles.push(Particle::new(
            position,
            nalgebra::Vector2::zeros(),
            T::one(),
            friction,
            false,
        ));
    }

    for i in 0..num_sides {
        polygon.constraints.push(Box::new(DistanceConstraint::new(
            i,
            (i + 1) % num_sides,
            &polygon.particles,
            stiffness_distance,
        )));
    }

    for i in 0..num_sides {
        polygon.constraints.push(Box::new(DistanceConstraint::new(
            i,
            (i + (num_sides / 2)) % num_sides,
            &polygon.particles,
            stiffness_shear,
        )));
    }

    polygon.constraints.push(Box::new(AreaConstraint::new(
        &polygon.particles,
        stiffness_area,
    )));

    polygon
}

pub fn create_rect<T: FloatingPointNumber>(
    start: Vector2<T>,
    width: T,
    height: T,
    stiffness_distance: T,
    stiffness_shear: T,
    stiffness_area: T,
    friction: T,
) -> Body<T> {
    let mut rect = Body::empty();
    let mut create_particle_helper = |offset| {
        rect.particles.push(Particle::new(
            start + offset,
            nalgebra::Vector2::zeros(),
            T::one(),
            friction,
            false,
        ));
    };
    create_particle_helper(Vector2::new(T::zero(), T::zero()));
    create_particle_helper(Vector2::new(width, T::zero()));
    create_particle_helper(Vector2::new(width, height));
    create_particle_helper(Vector2::new(T::zero(), height));

    let mut create_distance_constraint_helper = |idx_a, idx_b, stiffness| {
        rect.constraints.push(Box::new(DistanceConstraint::new(
            idx_a,
            idx_b,
            &rect.particles,
            stiffness,
        )));
    };

    for i in 0..4 {
        create_distance_constraint_helper(i, (i + 1) % 4, stiffness_distance);
    }
    for i in 0..2 {
        create_distance_constraint_helper(i, (i + 2) % 4, stiffness_shear);
    }

    rect.constraints.push(Box::new(AreaConstraint::new(
        &rect.particles,
        stiffness_area,
    )));
    rect
}

pub fn create_quad<T: FloatingPointNumber>(
    start: Vector2<T>,
    size: T,
    stiffness_distance: T,
    stiffness_shear: T,
    stiffness_area: T,
    friction: T,
) -> Body<T> {
    create_rect(
        start,
        size,
        size,
        stiffness_distance,
        stiffness_shear,
        stiffness_area,
        friction,
    )
}
