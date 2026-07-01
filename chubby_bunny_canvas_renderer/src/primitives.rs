use chubby_bunny_core::particle::Particle;
use chubby_bunny_core::{AreaConstraint, Body, DistanceConstraint};
use chubby_bunny_core::{BendingConstraint, FloatingPointNumber};
use nalgebra::Vector2;

pub struct SimpleBodySettings<T> {
    pub stiffness_distance: T,
    pub stiffness_shear: T,
    pub stiffness_area: T,
    pub stiffness_bending: T,
    pub friction: T,
}

/// Helper function to create a polygon body with specified parameters, including the center position, radius, number of sides, stiffness for various constraints,
/// and friction. The function generates particles arranged in a circular pattern and applies distance, shear, bending, and area constraints to maintain the shape and behavior of the polygon.
/// Shear constraints are applied between opposite corners to help resist deformation, while the area constraint helps maintain the overall area of the polygon.
pub fn create_polygon<T: FloatingPointNumber>(
    center: Vector2<T>,
    radius: T,
    num_sides: usize,
    settings: &SimpleBodySettings<T>,
) -> Body<T> {
    let mut polygon = Body::empty();
    for i in 0..num_sides {
        let angle = (i as f32 / num_sides as f32) * std::f32::consts::TAU;
        let position = center + Vector2::new(T::from(angle.cos()), T::from(angle.sin())) * radius;
        polygon.particles.push(Particle::new(
            position,
            nalgebra::Vector2::zeros(),
            T::one(),
            settings.friction,
            false,
        ));
    }

    for i in 0..num_sides {
        polygon.constraints.push(Box::new(DistanceConstraint::new(
            i,
            (i + 1) % num_sides,
            &polygon.particles,
            settings.stiffness_distance,
        )));
    }

    for i in 0..num_sides {
        polygon.constraints.push(Box::new(DistanceConstraint::new(
            i,
            (i + (num_sides / 2)) % num_sides,
            &polygon.particles,
            settings.stiffness_shear,
        )));
    }

    for i in 0..num_sides {
        polygon.constraints.push(Box::new(BendingConstraint::new(
            i,
            &polygon.particles,
            settings.stiffness_bending,
        )));
    }

    polygon.constraints.push(Box::new(AreaConstraint::new(
        &polygon.particles,
        settings.stiffness_area,
    )));

    polygon
}

/// Creates a rectangular body with specified parameters.
/// Shear constraints are applied between opposite corners to help resist deformation.
pub fn create_rect<T: FloatingPointNumber>(
    start: Vector2<T>,
    width: T,
    height: T,
    settings: &SimpleBodySettings<T>,
) -> Body<T> {
    let mut rect = Body::empty();
    let mut create_particle_helper = |offset| {
        rect.particles.push(Particle::new(
            start + offset,
            nalgebra::Vector2::zeros(),
            T::one(),
            settings.friction,
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
        create_distance_constraint_helper(i, (i + 1) % 4, settings.stiffness_distance);
    }
    for i in 0..2 {
        create_distance_constraint_helper(i, (i + 2) % 4, settings.stiffness_shear);
    }

    rect.constraints.push(Box::new(AreaConstraint::new(
        &rect.particles,
        settings.stiffness_area,
    )));
    rect
}

/// Creates a quadrilateral body with specified parameters.
/// Shear constraints are applied between opposite corners to help resist deformation.
pub fn create_quad<T: FloatingPointNumber>(
    start: Vector2<T>,
    size: T,
    stiffness_distance: T,
    stiffness_shear: T,
    stiffness_area: T,
    friction: T,
) -> Body<T> {
    let settings = SimpleBodySettings {
        stiffness_distance,
        stiffness_shear,
        stiffness_area,
        stiffness_bending: T::zero(),
        friction,
    };
    create_rect(start, size, size, &settings)
}
