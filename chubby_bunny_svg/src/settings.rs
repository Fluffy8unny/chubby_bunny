use chubby_bunny_core::FloatingPointNumber;

pub struct ParticleSettings<T> {
    pub mass: T,
    pub friction: T,
    pub is_static: bool,
}

pub struct ConstraintSettings<T> {
    pub stiffness_distance: T,
    pub stiffness_shear: T,
    pub stiffness_bending: T,
    pub stiffness_area: T,
    pub attachment_stiffness: T,
}

pub struct AttachmentSettings<T> {
    pub child_sample_stride: usize,
    pub max_total_attachments: usize,
    pub max_distance_factor: T,
    pub parent_springs_per_child_anchor: usize,
}

pub struct BodySettings<T> {
    pub particle_settings: ParticleSettings<T>,
}

pub struct SVGConstraintSettings<T> {
    pub constraint_settings: ConstraintSettings<T>,
    pub attachment_settings: AttachmentSettings<T>,
}

impl<T: FloatingPointNumber> BodySettings<T> {
    pub fn from_values(
        mass: T,
        friction: T,
        is_static: bool,
    ) -> Self {
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
