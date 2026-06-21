use chubby_bunny_core::FloatingPointNumber;

/// How to set up particles, that are being parsed from SVG files.
pub struct ParticleSettings<T> {
    /// Mass of each particle. Higher mass means more inertia and resistance to movement.
    pub mass: T,
    /// Friction coefficient for each particle, which determines how quickly it loses velocity over time.
    pub friction: T,
    /// Whether the particles should be static (pinned in place) or dynamic (able to move).
    pub is_static: bool,
}

/// Settings for automatic constraint generation, based to SVG polygon shapes.
pub struct ConstraintSettings<T> {
    /// Stiffness for DistanceConstraint, this is for the outline of the body.
    pub stiffness_distance: T,
    /// Stiffness for shear DistanceConstraints, that connect non-adjacent particles in the polygon. I.e. diagonals in a quad.
    pub stiffness_shear: T,
    /// Stiffness for bending constraints, which resist changes in the angle between adjacent edges.
    pub stiffness_bending: T,
    /// Stiffness for area constraints, which maintain the area of the polygon.
    pub stiffness_area: T,
    /// Stiffness for attachment constraints, which connect child bodies to parent bodies.
    pub attachment_stiffness: T,
}

/// Settings for how child bodies should be attached to parent bodies when parsing from SVG files.
pub struct AttachmentSettings<T> {
    /// The stride for sampling child particles when creating attachment constraints.
    pub child_sample_stride: usize,
    /// The maximum number of attachments allowed for a child body.
    pub max_total_attachments: usize,
    /// The maximum distance factor for creating attachment constraints.
    pub max_distance_factor: T,
    /// The number of parent springs per child anchor. We choose multiple attachment points on the parent for stability.
    pub parent_springs_per_child_anchor: usize,
}

pub struct BodySettings<T> {
    pub particle_settings: ParticleSettings<T>,
}

pub struct SVGConstraintSettings<T> {
    /// Settings for automatically generating constraints based on SVG polygon shapes.
    pub constraint_settings: ConstraintSettings<T>,
    /// Settings for how child bodies should be attached to parent bodies when parsing from SVG files.
    pub attachment_settings: AttachmentSettings<T>,
}

impl<T: FloatingPointNumber> BodySettings<T> {
    pub fn from_values(mass: T, friction: T, is_static: bool) -> Self {
        Self {
            particle_settings: ParticleSettings {
                mass,
                friction,
                is_static,
            },
        }
    }
}

impl<T: FloatingPointNumber> SVGConstraintSettings<T> {
    pub fn from_values(
        stiffness_distance: T,
        stiffness_shear: T,
        stiffness_bending: T,
        stiffness_area: T,
        attachment_stiffness: T,
        child_sample_stride: usize,
        max_total_attachments: usize,
        max_distance_factor: T,
        parent_springs_per_child_anchor: usize,
    ) -> Self {
        Self {
            constraint_settings: ConstraintSettings {
                stiffness_distance,
                stiffness_shear,
                stiffness_bending,
                stiffness_area,
                attachment_stiffness,
            },
            attachment_settings: AttachmentSettings {
                child_sample_stride,
                max_total_attachments,
                max_distance_factor,
                parent_springs_per_child_anchor,
            },
        }
    }
}
