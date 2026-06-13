use super::js_types::{BodyMeta, Color};
use chubby_bunny::{
    AreaConstraint, AttachmentConstraint, BendingConstraint, Body, BodyId, DistanceConstraint,
    ExtrinsicConstraintType, FloatingPointNumber, Particle,
};
use nalgebra::Vector2;
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub struct ParticleSettings<T: FloatingPointNumber> {
    pub mass: T,
    pub friction: T,
    pub is_static: bool,
}
pub struct ConstraintSettings<T: FloatingPointNumber> {
    pub stiffness_distance: T,
    pub stiffness_shear: T,
    pub stiffness_bending: T,
    pub stiffness_area: T,
    pub attachment_stiffness: T,
}
pub struct AttachmentSettings<T: FloatingPointNumber> {
    pub child_sample_stride: usize,
    pub max_total_attachments: usize,
    pub max_distance_factor: T,
    pub parent_springs_per_child_anchor: usize,
}

impl<T: FloatingPointNumber> Default for AttachmentSettings<T> {
    fn default() -> Self {
        Self {
            child_sample_stride: 4,
            max_total_attachments: 12,
            max_distance_factor: T::from(2.0),
            parent_springs_per_child_anchor: 3,
        }
    }
}

pub struct BodySettings<T: FloatingPointNumber> {
    pub particle_settings: ParticleSettings<T>,
    pub constraint_settings: ConstraintSettings<T>,
    pub attachment_settings: AttachmentSettings<T>,
}

impl<T: FloatingPointNumber> BodySettings<T> {
    pub fn from_values(
        mass: T,
        friction: T,
        is_static: bool,
        stiffness_distance: T,
        stiffness_shear: T,
        stiffness_bending: T,
        stiffness_area: T,
        attachment_stiffness: T,
    ) -> Self {
        Self {
            particle_settings: ParticleSettings {
                mass,
                friction,
                is_static,
            },
            constraint_settings: ConstraintSettings {
                stiffness_distance,
                stiffness_shear,
                stiffness_bending,
                stiffness_area,
                attachment_stiffness,
            },
            attachment_settings: AttachmentSettings::default(),
        }
    }
}

/// Root SVG element.
#[derive(Debug, Deserialize)]
#[serde(rename = "svg")]
struct Svg {
    #[serde(rename = "$value", default)]
    pub children: Vec<SvgNode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SvgNode {
    G(Group),
    Path(SvgPath),
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
struct Group {
    #[serde(rename = "$value", default)]
    pub children: Vec<SvgNode>,
}

#[derive(Debug, Deserialize)]
struct SvgPath {
    #[serde(rename = "@id")]
    pub _id: Option<String>,
    #[serde(rename = "@d")]
    pub d: Option<String>,
    #[serde(rename = "@style")]
    pub style: Option<String>,
}

fn parse_hex_color(input: &str) -> Option<(u8, u8, u8)> {
    let value = input.trim();
    if !value.starts_with('#') {
        return None;
    }

    let hex = &value[1..];
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some((r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some((r, g, b))
        }
        _ => None,
    }
}

fn parse_style_to_body_meta(style: &str, id: BodyId, z_index: i32) -> BodyMeta {
    let mut line_color = (0u8, 0u8, 0u8);
    let mut fill_color = (0u8, 0u8, 0u8);
    let mut line_alpha = 1.0f32;
    let mut fill_alpha = 1.0f32;
    let mut global_alpha = 1.0f32;
    let mut line_weight = 1.0f32;

    for item in style.split(';') {
        let mut kv = item.splitn(2, ':');
        let key = kv.next().map(str::trim).unwrap_or("");
        let value = kv.next().map(str::trim).unwrap_or("");

        match key {
            "stroke" => {
                if value == "none" {
                    line_alpha = 0.0;
                } else if let Some(rgb) = parse_hex_color(value) {
                    line_color = rgb;
                }
            }
            "fill" => {
                if value == "none" {
                    fill_alpha = 0.0;
                } else if let Some(rgb) = parse_hex_color(value) {
                    fill_color = rgb;
                }
            }
            "stroke-opacity" => {
                if let Ok(v) = value.parse::<f32>() {
                    line_alpha = v.clamp(0.0, 1.0);
                }
            }
            "fill-opacity" => {
                if let Ok(v) = value.parse::<f32>() {
                    fill_alpha = v.clamp(0.0, 1.0);
                }
            }
            "opacity" => {
                if let Ok(v) = value.parse::<f32>() {
                    global_alpha = v.clamp(0.0, 1.0);
                }
            }
            "stroke-width" => {
                if let Ok(v) = value.parse::<f32>() {
                    line_weight = v.max(0.0);
                }
            }
            _ => {}
        }
    }

    BodyMeta {
        id,
        z_index,
        line_weight,
        line_color: Color {
            r: line_color.0,
            g: line_color.1,
            b: line_color.2,
            a: line_alpha * global_alpha,
        },
        fill_color: Color {
            r: fill_color.0,
            g: fill_color.1,
            b: fill_color.2,
            a: fill_alpha * global_alpha,
        },
    }
}

fn tokenize_path_data(d: &str) -> Vec<String> {
    let mut normalized = String::with_capacity(d.len() * 2);
    for ch in d.chars() {
        if ch.is_ascii_alphabetic() {
            normalized.push(' ');
            normalized.push(ch);
            normalized.push(' ');
        } else if ch == ',' {
            normalized.push(' ');
        } else {
            normalized.push(ch);
        }
    }

    normalized
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

fn parse_single<T: FloatingPointNumber>(tokens: &[String], i: usize) -> Option<(T, usize)> {
    let v = T::from(tokens.get(i)?.parse::<f32>().ok()?);
    Some((v, i + 1))
}

fn parse_xy_pair<T: FloatingPointNumber>(
    tokens: &[String],
    i: usize,
) -> Option<(Vector2<T>, usize)> {
    let x = T::from(tokens.get(i)?.parse::<f32>().ok()?);
    let y = T::from(tokens.get(i + 1)?.parse::<f32>().ok()?);
    Some((Vector2::new(x, y), i + 2))
}

fn parse_simple_polygon_path<T: FloatingPointNumber>(d: &str) -> Vec<Vector2<T>> {
    let tokens = tokenize_path_data(d);
    let mut i = 0usize;
    let mut command = 'M';
    let mut current = Vector2::new(T::zero(), T::zero());
    let mut points: Vec<Vector2<T>> = Vec::new();

    while i < tokens.len() {
        match tokens[i].as_str() {
            "M" | "m" | "L" | "l" | "H" | "h" | "V" | "v" | "Z" | "z" => {
                command = tokens[i].chars().next().unwrap_or('M');
                i += 1;
                if command == 'Z' || command == 'z' {
                    break;
                }
                continue;
            }
            _ => {}
        }

        match command {
            'M' => {
                if let Some((p, next_i)) = parse_xy_pair::<T>(&tokens, i) {
                    current = p;
                    points.push(current);
                    i = next_i;
                    command = 'L';
                } else {
                    break;
                }
            }
            'm' => {
                if let Some((p, next_i)) = parse_xy_pair::<T>(&tokens, i) {
                    current = Vector2::new(current.x + p.x, current.y + p.y);
                    points.push(current);
                    i = next_i;
                    command = 'l';
                } else {
                    break;
                }
            }
            'L' => {
                if let Some((p, next_i)) = parse_xy_pair::<T>(&tokens, i) {
                    current = p;
                    points.push(current);
                    i = next_i;
                } else {
                    break;
                }
            }
            'l' => {
                if let Some((p, next_i)) = parse_xy_pair::<T>(&tokens, i) {
                    current = Vector2::new(current.x + p.x, current.y + p.y);
                    points.push(current);
                    i = next_i;
                } else {
                    break;
                }
            }
            'H' => {
                if let Some((x, next_i)) = parse_single::<T>(&tokens, i) {
                    current = Vector2::new(x, current.y);
                    points.push(current);
                    i = next_i;
                } else {
                    break;
                }
            }
            'h' => {
                if let Some((x, next_i)) = parse_single::<T>(&tokens, i) {
                    current = Vector2::new(current.x + x, current.y);
                    points.push(current);
                    i = next_i;
                } else {
                    break;
                }
            }
            'V' => {
                if let Some((y, next_i)) = parse_single::<T>(&tokens, i) {
                    current = Vector2::new(current.x, y);
                    points.push(current);
                    i = next_i;
                } else {
                    break;
                }
            }
            'v' => {
                if let Some((y, next_i)) = parse_single::<T>(&tokens, i) {
                    current = Vector2::new(current.x, current.y + y);
                    points.push(current);
                    i = next_i;
                } else {
                    break;
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    points
}

fn bbox_origin<T: FloatingPointNumber>(points: &[Vector2<T>]) -> Vector2<T> {
    let mut iter = points.iter();
    let first = iter.next().copied().unwrap_or_else(Vector2::zeros);
    iter.fold(first, |acc, p| Vector2::new(acc.x.min(p.x), acc.y.min(p.y)))
}

fn normalize_points_to_anchor<T: FloatingPointNumber>(
    points: Vec<Vector2<T>>,
    anchor: Vector2<T>,
) -> Vec<Vector2<T>> {
    points
        .into_iter()
        .map(|p| Vector2::new(p.x - anchor.x, p.y - anchor.y))
        .collect()
}

fn add_shape_aware_shear_constraints<T: FloatingPointNumber>(body: &mut Body<T>, stiffness: T) {
    let n = body.particles.len();
    if n < 4 || stiffness <= T::zero() {
        return;
    }

    // Deterministic long-span shear links with balanced opposite picks.
    // For odd polygons, alternating floor/ceil opposite offsets avoids a
    // directional bias that can push local points outward.
    let opposite_step_lo = (n / 2).max(2);
    let opposite_step_hi = ((n + 1) / 2).max(2);
    let mut seen_pairs: HashSet<(usize, usize)> = HashSet::new();

    for i in 0..n {
        let step = if n % 2 == 0 {
            opposite_step_lo
        } else if i % 2 == 0 {
            opposite_step_lo
        } else {
            opposite_step_hi
        };
        let j = (i + step) % n;
        if i == j {
            continue;
        }

        let cw = (j + n - i) % n;
        let ccw = (i + n - j) % n;
        let ring_distance = cw.min(ccw);
        if ring_distance <= 1 {
            continue;
        }

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

fn add_boundary_bending_constraints<T: FloatingPointNumber>(body: &mut Body<T>, stiffness: T) {
    let n = body.particles.len();
    if n < 3 || stiffness <= T::zero() {
        return;
    }

    for i in 0..n {
        let prev = (i + n - 1) % n;
        let next = (i + 1) % n;
        if prev == next {
            continue;
        }

        body.constraints.push(Rc::new(BendingConstraint::new(
            prev,
            i,
            next,
            &body.particles,
            stiffness,
        )));
    }
}

fn parse_svg_path_to_body<T: FloatingPointNumber>(
    path: &SvgPath,
    z_index: i32,
    anchor: Option<Vector2<T>>,
    settings: &BodySettings<T>,
) -> Option<(Body<T>, BodyMeta, Vector2<T>)> {
    let d = path.d.as_deref()?;
    let points = parse_simple_polygon_path::<T>(d);
    if points.len() < 3 {
        return None;
    }
    let anchor = anchor.unwrap_or_else(|| bbox_origin::<T>(&points));
    let points = normalize_points_to_anchor(points, anchor);

    let mut body = Body::empty();
    for point in points {
        body.particles.push(Particle::new(
            point,
            Vector2::zeros(),
            settings.particle_settings.mass,
            settings.particle_settings.friction,
            settings.particle_settings.is_static,
        ));
    }

    for i in 0..body.particles.len() {
        body.constraints.push(Rc::new(DistanceConstraint::new(
            i,
            (i + 1) % body.particles.len(),
            &body.particles,
            settings.constraint_settings.stiffness_distance,
        )));
    }

    let idxs: Vec<usize> = (0..body.particles.len()).collect();
    body.constraints.push(Rc::new(AreaConstraint::new(
        idxs,
        &body.particles,
        settings.constraint_settings.stiffness_area,
    )));

    add_boundary_bending_constraints(&mut body, settings.constraint_settings.stiffness_bending);
    add_shape_aware_shear_constraints(&mut body, settings.constraint_settings.stiffness_shear);

    let style = path.style.as_deref().unwrap_or("");
    let meta = parse_style_to_body_meta(style, body.id, z_index);

    Some((body, meta, anchor))
}

fn parse_svg_to_hierarchy<T: FloatingPointNumber>(
    svg: &Svg,
    settings: &BodySettings<T>,
) -> (Vec<Body<T>>, HashMap<BodyId, BodyMeta>) {
    let mut meta_map = HashMap::new();
    let bodies = parse_nodes_recursive(&svg.children, 0, None, &mut meta_map, settings);
    (bodies, meta_map)
}

fn parse_nodes_recursive<T: FloatingPointNumber>(
    nodes: &[SvgNode],
    z_index: i32,
    anchor: Option<Vector2<T>>,
    meta_map: &mut HashMap<BodyId, BodyMeta>,
    settings: &BodySettings<T>,
) -> Vec<Body<T>> {
    let mut bodies = Vec::new();
    for node in nodes {
        match node {
            SvgNode::Path(path) => {
                if let Some((body, meta, _anchor)) =
                    parse_svg_path_to_body(path, z_index, anchor.clone(), settings)
                {
                    meta_map.insert(body.id, meta);
                    bodies.push(body);
                }
            }
            SvgNode::G(group) => {
                bodies.extend(parse_group_recursive(
                    group,
                    z_index + 1,
                    anchor.clone(),
                    meta_map,
                    settings,
                ));
            }
            SvgNode::Unknown => {}
        }
    }
    bodies
}

fn nearest_parent_attachment_points<T: FloatingPointNumber>(
    parent: &Body<T>,
    child: &Body<T>,
    settings: &AttachmentSettings<T>,
) -> (Vec<usize>, Vec<usize>) {
    if parent.particles.is_empty() || child.particles.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let stride = settings.child_sample_stride.max(1);
    let sampled_child_indices: Vec<usize> = (0..child.particles.len()).step_by(stride).collect();
    if sampled_child_indices.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let selected_child_indices = if settings.max_total_attachments > 0
        && sampled_child_indices.len() > settings.max_total_attachments
    {
        let mut out = Vec::with_capacity(settings.max_total_attachments);
        for k in 0..settings.max_total_attachments {
            let mapped = (k * sampled_child_indices.len()) / settings.max_total_attachments;
            let idx = sampled_child_indices[mapped];
            if out.last().copied() != Some(idx) {
                out.push(idx);
            }
        }
        out
    } else {
        sampled_child_indices
    };

    let parent_centroid = parent
        .particles
        .iter()
        .fold(Vector2::zeros(), |acc, p| acc + p.position)
        / T::from(parent.particles.len() as f32);

    let mut candidates: Vec<(usize, usize, T)> = Vec::new();
    let mut candidate_distances_sq: Vec<T> = Vec::new();

    for child_idx in selected_child_indices {
        let child_pos = child.particles[child_idx].position;
        let child_vec = child_pos - parent_centroid;
        let child_norm = child_vec.norm();

        let mut best_parent_idx = 0usize;
        let mut best_score = T::max_value().unwrap_or(T::from(1.0e12));
        let mut best_dist_sq =
            (parent.particles[0].position - child.particles[child_idx].position).norm_squared();

        for (parent_idx, parent_particle) in parent.particles.iter().enumerate() {
            let parent_pos = parent_particle.position;
            let dist_sq = (parent_pos - child_pos).norm_squared();

            let score = if child_norm <= T::from(1.0e-6_f32) {
                // Near centroid: fallback to nearest geometric neighbor.
                dist_sq
            } else {
                let parent_vec = parent_pos - parent_centroid;
                let parent_norm = parent_vec.norm();
                if parent_norm <= T::from(1.0e-6_f32) {
                    T::max_value().unwrap_or(T::from(1.0e12))
                } else {
                    let alignment = child_vec.dot(&parent_vec) / (child_norm * parent_norm);
                    let radial_gap = parent_norm - child_norm;
                    let angle_term = T::one() - alignment;
                    let radial_penalty = if radial_gap > T::zero() {
                        radial_gap * T::from(0.15_f32)
                    } else {
                        (-radial_gap) * T::from(2.0_f32) + T::from(1.0_f32)
                    };
                    let distance_penalty = dist_sq * T::from(0.01_f32);
                    angle_term + radial_penalty + distance_penalty
                }
            };

            if score < best_score {
                best_score = score;
                best_parent_idx = parent_idx;
                best_dist_sq = dist_sq;
            }
        }

        candidates.push((best_parent_idx, child_idx, best_dist_sq));
        candidate_distances_sq.push(best_dist_sq);
    }

    if candidates.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut all_candidates_by_distance = candidates.clone();
    all_candidates_by_distance.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal));

    candidate_distances_sq.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let len = candidate_distances_sq.len();
    let median_distance_sq = if len % 2 == 0 {
        (candidate_distances_sq[len / 2 - 1] + candidate_distances_sq[len / 2]) / T::from(2.0)
    } else {
        candidate_distances_sq[len / 2]
    };

    // Avoid over-pruning tiny anchor sets. Losing one of 3 anchors can destabilize pose recovery.
    if candidates.len() > 4 {
        let max_distance_factor_sq = settings.max_distance_factor * settings.max_distance_factor;
        let max_distance_sq = median_distance_sq * max_distance_factor_sq;
        candidates.retain(|(_, _, distance_sq)| *distance_sq <= max_distance_sq);
    }

    candidates.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal));

    // Keep at least 3 anchors when available.
    let min_kept = candidates
        .len()
        .max(all_candidates_by_distance.len().min(3));
    if candidates.len() < min_kept {
        for fallback in all_candidates_by_distance {
            if candidates.len() >= min_kept {
                break;
            }
            if !candidates.iter().any(|c| c.1 == fallback.1) {
                candidates.push(fallback);
            }
        }
        candidates.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal));
    }

    let mut parent_idxs = Vec::new();
    let mut child_idxs = Vec::new();
    let parent_len = parent.particles.len();
    let mut seen_pairs: HashSet<(usize, usize)> = HashSet::new();
    let springs_per_child = settings.parent_springs_per_child_anchor.max(1);
    for (base_parent_idx, child_idx, _) in candidates {
        let mut support_parent_idxs = Vec::new();
        if parent_len > 0 {
            for i in 0..springs_per_child {
                let idx = (base_parent_idx + (i * parent_len) / springs_per_child) % parent_len;
                support_parent_idxs.push(idx);
            }
        }

        support_parent_idxs.sort_unstable();
        support_parent_idxs.dedup();

        for parent_idx in support_parent_idxs {
            if seen_pairs.insert((parent_idx, child_idx)) {
                parent_idxs.push(parent_idx);
                child_idxs.push(child_idx);
            }
        }
    }

    (parent_idxs, child_idxs)
}

fn parse_group_recursive<T: FloatingPointNumber>(
    group: &Group,
    z_index: i32,
    anchor: Option<Vector2<T>>,
    meta_map: &mut HashMap<BodyId, BodyMeta>,
    settings: &BodySettings<T>,
) -> Vec<Body<T>> {
    let (paths_and_others, child_groups_and_others): (Vec<_>, Vec<_>) = group
        .children
        .iter()
        .partition(|node| matches!(node, SvgNode::Path(_)));

    let direct_paths: Vec<_> = paths_and_others
        .into_iter()
        .filter_map(|node| match node {
            SvgNode::Path(path) => Some(path),
            _ => None,
        })
        .collect();
    let child_groups: Vec<_> = child_groups_and_others
        .into_iter()
        .filter_map(|node| match node {
            SvgNode::G(child_group) => Some(child_group),
            _ => None,
        })
        .collect();

    // If a group has no direct path, bubble child group bodies up.
    if direct_paths.is_empty() {
        return child_groups
            .into_iter()
            .flat_map(|child_group| {
                parse_group_recursive(child_group, z_index + 1, anchor.clone(), meta_map, settings)
            })
            .collect();
    }

    let mut bodies = Vec::new();
    for path in direct_paths {
        if let Some((mut body, meta, body_anchor)) =
            parse_svg_path_to_body(path, z_index, anchor.clone(), settings)
        {
            meta_map.insert(body.id, meta);

            let mut parsed_children = Vec::new();
            for child_group in &child_groups {
                parsed_children.extend(parse_group_recursive(
                    child_group,
                    z_index + 1,
                    Some(body_anchor.clone()),
                    meta_map,
                    settings,
                ));
            }

            for child in parsed_children {
                let (parent_idxs, child_idxs) =
                    nearest_parent_attachment_points(&body, &child, &settings.attachment_settings);
                if !parent_idxs.is_empty() {
                    body.children_constraints
                        .push(ExtrinsicConstraintType::Local(Box::new(
                            AttachmentConstraint::new(
                                child.id,
                                &body,
                                &child,
                                parent_idxs,
                                child_idxs,
                                settings.constraint_settings.attachment_stiffness,
                            ),
                        )));
                }
                body.children.push(child);
            }

            bodies.push(body);
        }
    }

    bodies
}

pub fn load_svg<T: FloatingPointNumber>(
    xml: &str,
    settings: &BodySettings<T>,
) -> (Vec<Body<T>>, HashMap<BodyId, BodyMeta>) {
    let svg: Svg = quick_xml::de::from_str(xml)
        .expect("Failed to parse SVG XML. Ensure the input is a valid SVG string.");
    parse_svg_to_hierarchy(&svg, settings)
}
