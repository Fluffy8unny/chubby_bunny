use crate::meta::{BodyMeta, Color};
use crate::svg_constraints::{
    add_boundary_bending_constraints, add_boundary_distance_constraints,
    add_skip_shear_constraints, nearest_parent_attachment_points,
};
use chubby_bunny_core::{
    body, AreaConstraint, AttachmentConstraint, Body, BodyId, ExtrinsicConstraintType,
    FloatingPointNumber, Particle, Transformation,
};
use nalgebra::Vector2;
use serde::Deserialize;
use std::collections::HashMap;
use std::rc::Rc;

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

pub struct BodySettings<T> {
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
        smooth_edges: true,
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

fn accumulate_bbox_recursive<T: FloatingPointNumber>(
    body: &Body<T>,
    min: &mut Vector2<T>,
    max: &mut Vector2<T>,
) {
    for particle in body.particles.iter() {
        min.x = min.x.min(particle.position.x);
        min.y = min.y.min(particle.position.y);
        max.x = max.x.max(particle.position.x);
        max.y = max.y.max(particle.position.y);
    }

    for child in body.children.iter() {
        accumulate_bbox_recursive(child, min, max);
    }
}

fn normalized_template_transform<T: FloatingPointNumber>(bodies: &[Body<T>]) -> Transformation<T> {
    let mut min = Vector2::new(T::max_value().unwrap(), T::max_value().unwrap());
    let mut max = Vector2::new(T::min_value().unwrap(), T::min_value().unwrap());
    let mut has_points = false;

    for body in bodies.iter() {
        if !body.particles.is_empty() {
            has_points = true;
        }
        accumulate_bbox_recursive(body, &mut min, &mut max);
    }

    if !has_points {
        return Transformation::identity();
    }

    let size = max - min;
    let max_extent = size.x.max(size.y);
    let scale = if max_extent > T::zero() {
        T::one() / max_extent
    } else {
        T::one()
    };

    Transformation {
        offset: Vector2::new(-min.x * scale, -min.y * scale),
        scale,
        rotation_radians: T::zero(),
    }
}

fn collect_id_pairs_recursive<T>(
    template: &Body<T>,
    instance: &Body<T>,
    id_pairs: &mut Vec<(BodyId, BodyId)>,
) {
    id_pairs.push((template.id, instance.id));
    for (template_child, instance_child) in template.children.iter().zip(instance.children.iter()) {
        collect_id_pairs_recursive(template_child, instance_child, id_pairs);
    }
}

pub fn instantiate_svg_body<T: FloatingPointNumber>(
    template: &Body<T>,
    template_meta: &HashMap<BodyId, BodyMeta>,
    transformation: Transformation<T>,
) -> (Body<T>, HashMap<BodyId, BodyMeta>) {
    let instance = template.duplicate_with_transformation(transformation);
    let mut instance_meta = HashMap::new();

    let mut id_pairs = Vec::new();
    collect_id_pairs_recursive(template, &instance, &mut id_pairs);
    for (template_id, instance_id) in id_pairs {
        if let Some(meta) = template_meta.get(&template_id) {
            let mut copied_meta = meta.clone();
            copied_meta.id = instance_id;
            instance_meta.insert(instance_id, copied_meta);
        }
    }

    (instance, instance_meta)
}

pub fn instantiate_svg_bodies<T: FloatingPointNumber>(
    templates: &[Body<T>],
    template_meta: &HashMap<BodyId, BodyMeta>,
    transformation: Transformation<T>,
) -> (Vec<Body<T>>, HashMap<BodyId, BodyMeta>) {
    let mut instances = Vec::with_capacity(templates.len());
    let mut instance_meta = HashMap::new();

    for template in templates.iter() {
        let (instance, meta) = instantiate_svg_body(template, template_meta, transformation);
        instances.push(instance);
        instance_meta.extend(meta);
    }

    (instances, instance_meta)
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

    add_boundary_distance_constraints(&mut body, settings.constraint_settings.stiffness_distance);

    let idxs: Vec<usize> = (0..body.particles.len()).collect();
    body.constraints.push(Rc::new(AreaConstraint::new(
        idxs,
        &body.particles,
        settings.constraint_settings.stiffness_area,
    )));

    add_boundary_bending_constraints(&mut body, settings.constraint_settings.stiffness_bending);
    add_skip_shear_constraints(&mut body, settings.constraint_settings.stiffness_shear);

    let style = path.style.as_deref().unwrap_or("");
    let meta = parse_style_to_body_meta(style, body.id, z_index);

    Some((body, meta, anchor))
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

fn parse_group_recursive<T: FloatingPointNumber>(
    group: &Group,
    z_index: i32,
    anchor: Option<Vector2<T>>,
    meta_map: &mut HashMap<BodyId, BodyMeta>,
    settings: &BodySettings<T>,
) -> Vec<Body<T>> {
    let direct_paths: Vec<&SvgPath> = group
        .children
        .iter()
        .filter_map(|node| match node {
            SvgNode::Path(path) => Some(path),
            _ => None,
        })
        .collect();
    let child_groups: Vec<&Group> = group
        .children
        .iter()
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
    let mut template_meta = HashMap::new();
    let templates = parse_nodes_recursive(&svg.children, 0, None, &mut template_meta, settings);
    let normalization = normalized_template_transform(&templates);
    instantiate_svg_bodies(&templates, &template_meta, normalization)
}
