use crate::svg::AttachmentSettings;
use chubby_bunny_core::{BendingConstraint, Body, DistanceConstraint, FloatingPointNumber};
use nalgebra::Vector2;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::rc::Rc;

pub fn add_boundary_distance_constraints<T: FloatingPointNumber>(body: &mut Body<T>, stiffness: T) {
    let n = body.particles.len();
    if n < 2 || stiffness <= T::zero() {
        return;
    }

    for i in 0..n {
        body.constraints.push(Rc::new(DistanceConstraint::new(
            i,
            (i + 1) % n,
            &body.particles,
            stiffness,
        )));
    }
}

pub fn add_skip_shear_constraints<T: FloatingPointNumber>(body: &mut Body<T>, stiffness: T) {
    let n = body.particles.len();
    if n < 4 || stiffness <= T::zero() {
        return;
    }

    // Two diagonal bands: n/3 (inner) and n/2 (cross-body).
    // Bending handles local angles and area handles volume; these two
    // bands provide medium and full cross-body bracing without
    // over-constraining the solver.
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
            body.constraints.push(Rc::new(DistanceConstraint::new(
                i,
                j,
                &body.particles,
                stiffness,
            )));
        }
    }
}

pub fn add_boundary_bending_constraints<T: FloatingPointNumber>(body: &mut Body<T>, stiffness: T) {
    let n = body.particles.len();
    if n < 3 || stiffness <= T::zero() {
        return;
    }

    for i in 0..n {
        let prev = (i + n - 1) % n;
        let next = (i + 1) % n;
        body.constraints.push(Rc::new(BendingConstraint::new(
            prev,
            i,
            next,
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

    if settings.max_total_attachments == 0 || sampled.len() <= settings.max_total_attachments {
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
    if child_norm <= T::from(1.0e-6_f32) {
        return dist_sq;
    }

    let parent_vec = parent_pos - parent_centroid;
    let parent_norm = parent_vec.norm();
    if parent_norm <= T::from(1.0e-6_f32) {
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

        let mut best_parent_idx = 0usize;
        let mut best_score = T::max_value().unwrap_or(T::from(1.0e12_f32));
        let mut best_dist_sq = (parent.particles[0].position - child_pos).norm_squared();

        for (parent_idx, parent_particle) in parent.particles.iter().enumerate() {
            let parent_pos = parent_particle.position;
            let score = parent_attachment_score(
                parent_pos,
                parent_centroid,
                child_pos,
                child_vec,
                child_norm,
            );

            if score < best_score {
                best_score = score;
                best_parent_idx = parent_idx;
                best_dist_sq = (parent_pos - child_pos).norm_squared();
            }
        }

        candidates.push(AttachmentCandidate {
            parent_idx: best_parent_idx,
            child_idx,
            dist_sq: best_dist_sq,
        });
    }

    candidates
}

fn prune_distance_outliers<T: FloatingPointNumber>(
    mut candidates: Vec<AttachmentCandidate<T>>,
    settings: &AttachmentSettings<T>,
) -> Vec<AttachmentCandidate<T>> {
    candidates.sort_by(|a, b| a.dist_sq.partial_cmp(&b.dist_sq).unwrap_or(Ordering::Equal));
    let fallback = candidates.clone();

    let len = candidates.len();
    let median_distance_sq = if len.is_multiple_of(2) {
        (candidates[len / 2 - 1].dist_sq + candidates[len / 2].dist_sq) / T::from(2.0)
    } else {
        candidates[len / 2].dist_sq
    };

    if candidates.len() > 4 {
        let max_distance_factor_sq = settings.max_distance_factor * settings.max_distance_factor;
        let max_distance_sq = median_distance_sq * max_distance_factor_sq;
        candidates.retain(|c| c.dist_sq <= max_distance_sq);
    }

    let min_kept = fallback.len().min(3);
    for candidate in fallback {
        if candidates.len() >= min_kept {
            break;
        }
        if !candidates
            .iter()
            .any(|c| c.child_idx == candidate.child_idx)
        {
            candidates.push(candidate);
        }
    }

    candidates.sort_by(|a, b| a.dist_sq.partial_cmp(&b.dist_sq).unwrap_or(Ordering::Equal));
    candidates
}

fn expand_candidates_to_springs<T: FloatingPointNumber>(
    candidates: &[AttachmentCandidate<T>],
    parent_len: usize,
    settings: &AttachmentSettings<T>,
) -> (Vec<usize>, Vec<usize>) {
    let mut parent_idxs = Vec::new();
    let mut child_idxs = Vec::new();
    let mut seen_pairs: HashSet<(usize, usize)> = HashSet::new();
    let springs_per_child = settings.parent_springs_per_child_anchor.max(1);

    for candidate in candidates {
        let mut support_parent_idxs = Vec::with_capacity(springs_per_child);
        if parent_len > 0 {
            for i in 0..springs_per_child {
                let idx =
                    (candidate.parent_idx + (i * parent_len) / springs_per_child) % parent_len;
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

fn parent_centroid_of<T: FloatingPointNumber>(parent: &Body<T>) -> Vector2<T> {
    parent
        .particles
        .iter()
        .fold(Vector2::zeros(), |acc, p| acc + p.position)
        / T::from(parent.particles.len() as f32)
}

pub fn nearest_parent_attachment_points<T: FloatingPointNumber>(
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

    let parent_centroid = parent_centroid_of(parent);
    let candidates = best_parent_per_child(parent, child, &child_indices, parent_centroid);
    if candidates.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let candidates = prune_distance_outliers(candidates, settings);
    expand_candidates_to_springs(&candidates, parent.particles.len(), settings)
}
