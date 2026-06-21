use crate::{AttachmentSettings, SVGConstraintSettings};
use chubby_bunny_core::{
    eps, AreaConstraint, AttachmentConstraint, BendingConstraint, Body, DistanceConstraint,
    ExtrinsicConstraintType, FloatingPointNumber,
};
use nalgebra::Vector2;
use std::cmp::Ordering;
use std::collections::HashSet;

/// Adds distance constraints between adjacent particles in the body to maintain the outer shape of the body.
/// This is doing nothing for internal stability, but is important for maintaining the visual shape of the body,
/// especially for bodies with few particles.
pub fn add_boundary_distance_constraints<T: FloatingPointNumber>(body: &mut Body<T>, stiffness: T) {
    let n = body.particles.len();
    if n < 2 || stiffness <= T::zero() {
        return;
    }

    for i in 0..n {
        body.constraints.push(Box::new(DistanceConstraint::new(
            i,
            (i + 1) % n,
            &body.particles,
            stiffness,
        )));
    }
}

/// Adds an area constraint to the body to maintain its overall area. Basically limits squishyness.
pub fn add_area_constraints<T: FloatingPointNumber>(body: &mut Body<T>, stiffness: T) {
    if body.particles.len() < 3 || stiffness <= T::zero() {
        return;
    }
    body.constraints
        .push(Box::new(AreaConstraint::new(&body.particles, stiffness)));
}

/// Adds shear constraints to the body to maintain its internal structure. These constraints
/// connect non-adjacent particles, helping to resist deformation.
/// Internally it adds distance constraints between particles that are 1/3 and 1/2 of the way around the body.
pub fn add_shear_constraints<T: FloatingPointNumber>(body: &mut Body<T>, stiffness: T) {
    let n = body.particles.len();
    if n < 4 || stiffness <= T::zero() {
        return;
    }

    let mut seen_pairs: HashSet<(usize, usize)> = HashSet::new();
    for step in [n / 3, n / 2] {
        if step < 2 {
            continue;
        }
        for i in 0..n {
            let j = (i + step) % n;
            let key = if i < j { (i, j) } else { (j, i) };
            if !seen_pairs.insert(key) {
                continue;
            }
            body.constraints.push(Box::new(DistanceConstraint::new(
                i,
                j,
                &body.particles,
                stiffness,
            )));
        }
    }
}

/// Adds bending constraints to the body to resist changes in the angle between adjacent edges.
/// This is important for concave regions, to prevent them from folding in on themselves.
pub fn add_boundary_bending_constraints<T: FloatingPointNumber>(body: &mut Body<T>, stiffness: T) {
    let n = body.particles.len();
    if n < 3 || stiffness <= T::zero() {
        return;
    }

    for i in 0..n {
        body.constraints.push(Box::new(BendingConstraint::new(
            i,
            &body.particles,
            stiffness,
        )));
    }
}

#[derive(Clone, Copy)]
struct AttachmentCandidate<T> {
    parent_idx: usize,
    child_idx: usize,
    dist_sq: T,
}

fn select_child_anchor_indices<T: FloatingPointNumber>(
    child: &Body<T>,
    settings: &AttachmentSettings<T>,
) -> Vec<usize> {
    let stride = settings.child_sample_stride.max(1);
    let sampled: Vec<usize> = (0..child.particles.len()).step_by(stride).collect();

    if sampled.len() <= settings.max_total_attachments {
        return sampled;
    }

    let mut out = Vec::with_capacity(settings.max_total_attachments);
    for k in 0..settings.max_total_attachments {
        let mapped = (k * sampled.len()) / settings.max_total_attachments;
        let idx = sampled[mapped];
        if out.last().copied() != Some(idx) {
            out.push(idx);
        }
    }
    out
}

fn parent_attachment_score<T: FloatingPointNumber>(
    parent_pos: Vector2<T>,
    parent_centroid: Vector2<T>,
    child_pos: Vector2<T>,
    child_vec: Vector2<T>,
    child_norm: T,
) -> T {
    let dist_sq = (parent_pos - child_pos).norm_squared();
    if child_norm <= eps!(T, 6) {
        return dist_sq;
    }

    let parent_vec = parent_pos - parent_centroid;
    let parent_norm = parent_vec.norm();
    if parent_norm <= eps!(T, 6) {
        return T::max_value().unwrap_or(T::from(1.0e12_f32));
    }

    let alignment = child_vec.dot(&parent_vec) / (child_norm * parent_norm);
    let angle_term = T::one() - alignment;
    dist_sq + angle_term * dist_sq * T::from(0.25_f32)
}

fn best_parent_per_child<T: FloatingPointNumber>(
    parent: &Body<T>,
    child: &Body<T>,
    child_indices: &[usize],
    parent_centroid: Vector2<T>,
) -> Vec<AttachmentCandidate<T>> {
    let mut candidates = Vec::with_capacity(child_indices.len());

    for &child_idx in child_indices {
        let child_pos = child.particles[child_idx].position;
        let child_vec = child_pos - parent_centroid;
        let child_norm = child_vec.norm();

        let best_parent = parent
            .particles
            .iter()
            .enumerate()
            .map(|(parent_idx, parent_particle)| {
                let parent_pos = parent_particle.position;
                let dist_sq = (parent_pos - child_pos).norm_squared();
                let score = parent_attachment_score(
                    parent_pos,
                    parent_centroid,
                    child_pos,
                    child_vec,
                    child_norm,
                );

                (
                    score,
                    AttachmentCandidate {
                        parent_idx,
                        child_idx,
                        dist_sq,
                    },
                )
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal))
            .map(|(_, candidate)| candidate);

        if let Some(candidate) = best_parent {
            candidates.push(candidate);
        }
    }

    candidates
}

fn median_for_sorted<T: FloatingPointNumber>(values: &[AttachmentCandidate<T>]) -> T {
    let len = values.len();
    if len == 0 {
        return T::zero();
    }
    if len % 2 == 1 {
        values[len / 2].dist_sq
    } else {
        (values[len / 2 - 1].dist_sq + values[len / 2].dist_sq) / T::from(2.0)
    }
}

fn prune_distance_outliers<T: FloatingPointNumber>(
    mut candidates: Vec<AttachmentCandidate<T>>,
    settings: &AttachmentSettings<T>,
) -> Vec<AttachmentCandidate<T>> {
    candidates.sort_by(|a, b| a.dist_sq.partial_cmp(&b.dist_sq).unwrap_or(Ordering::Equal));
    if candidates.len() <= 4 {
        return candidates;
    }

    let original = candidates.clone();
    let median_distance_sq = median_for_sorted(&original);

    let max_distance_factor_sq = settings.max_distance_factor * settings.max_distance_factor;
    let max_distance_sq = median_distance_sq * max_distance_factor_sq;

    let (mut kept, pruned): (Vec<_>, Vec<_>) = original
        .into_iter()
        .partition(|candidate| candidate.dist_sq <= max_distance_sq);

    if kept.len() < 3 {
        kept.extend(pruned.into_iter().take(3 - kept.len()));
        kept.sort_by(|a, b| a.dist_sq.partial_cmp(&b.dist_sq).unwrap_or(Ordering::Equal));
    }

    kept
}

fn expand_candidates_to_supports<T: FloatingPointNumber>(
    candidates: &[AttachmentCandidate<T>],
    parent_len: usize,
    settings: &AttachmentSettings<T>,
) -> (Vec<usize>, Vec<usize>) {
    let mut parent_idxs = Vec::new();
    let mut child_idxs = Vec::new();
    let mut seen_pairs: HashSet<(usize, usize)> = HashSet::new();
    let supports_per_child = settings.parent_springs_per_child_anchor.max(1);

    for candidate in candidates {
        let mut support_parent_idxs = Vec::with_capacity(supports_per_child);
        if parent_len > 0 {
            for i in 0..supports_per_child {
                let idx =
                    (candidate.parent_idx + (i * parent_len) / supports_per_child) % parent_len;
                support_parent_idxs.push(idx);
            }
        }

        support_parent_idxs.sort_unstable();
        support_parent_idxs.dedup();

        for parent_idx in support_parent_idxs {
            if seen_pairs.insert((parent_idx, candidate.child_idx)) {
                parent_idxs.push(parent_idx);
                child_idxs.push(candidate.child_idx);
            }
        }
    }

    (parent_idxs, child_idxs)
}

fn nearest_parent_attachment_points<T: FloatingPointNumber>(
    parent: &Body<T>,
    child: &Body<T>,
    settings: &AttachmentSettings<T>,
) -> (Vec<usize>, Vec<usize>) {
    if parent.particles.is_empty() || child.particles.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let child_indices = select_child_anchor_indices(child, settings);
    if child_indices.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let parent_centroid = parent.centroid();
    let candidates = best_parent_per_child(parent, child, &child_indices, parent_centroid);
    if candidates.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let candidates = prune_distance_outliers(candidates, settings);
    expand_candidates_to_supports(&candidates, parent.particles.len(), settings)
}

/// Attaches a child body to a parent body by creating attachment constraints between selected pairs of parent and child particles.
pub fn attach_child_to_parent<T: FloatingPointNumber>(
    parent: &mut Body<T>,
    child: &Body<T>,
    settings: &SVGConstraintSettings<T>,
) {
    let (parent_idxs, child_idxs) =
        nearest_parent_attachment_points(parent, child, &settings.attachment_settings);
    if !parent_idxs.is_empty() {
        parent
            .children_constraints
            .push(ExtrinsicConstraintType::Local(Box::new(
                AttachmentConstraint::new(
                    child.id,
                    parent,
                    child,
                    parent_idxs,
                    child_idxs,
                    settings.constraint_settings.attachment_stiffness,
                ),
            )));
    }
}
