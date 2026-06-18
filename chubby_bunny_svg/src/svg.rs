use crate::meta::{BodyMeta, Color};
use crate::svg_constraints::{
    add_boundary_bending_constraints, add_boundary_distance_constraints, add_shear_constraints,
    nearest_parent_attachment_points,
};
use chubby_bunny_core::{
    AreaConstraint, AttachmentConstraint, Body, BodyId, ExtrinsicConstraintType,
    FloatingPointNumber, Particle, Transformation,
};
use nalgebra::Vector2;
use serde::Deserialize;
use std::collections::HashMap;
use svgtypes::{Length, PathParser, PathSegment};

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
        child_sample_stride: usize,
        max_total_attachments: usize,
        max_distance_factor: T,
        parent_springs_per_child_anchor: usize,
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
            attachment_settings: AttachmentSettings {
                child_sample_stride,
                max_total_attachments,
                max_distance_factor,
                parent_springs_per_child_anchor,
            },
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

fn parse_style_to_body_meta(style: &str, id: BodyId, z_index: i32) -> BodyMeta {
    let mut line_color = Color::black();
    let mut fill_color = Color::black();
    let mut global_alpha = 1.0_f32;
    let mut line_weight = 1.0_f32;

    for item in style.split(';') {
        let mut kv = item.splitn(2, ':');
        let key = kv.next().map(str::trim).unwrap_or("");
        let value = kv.next().map(str::trim).unwrap_or("");

        match key {
            "stroke" => {
                if let Some(color) = Color::from_paint(value) {
                    line_color = color;
                }
            }
            "fill" => {
                if let Some(color) = Color::from_paint(value) {
                    fill_color = color;
                }
            }
            "stroke-opacity" => {
                if let Ok(v) = value.parse::<f32>() {
                    line_color.a = v.clamp(0.0, 1.0);
                }
            }
            "fill-opacity" => {
                if let Ok(v) = value.parse::<f32>() {
                    fill_color.a = v.clamp(0.0, 1.0);
                }
            }
            "opacity" => {
                if let Ok(v) = value.parse::<f32>() {
                    global_alpha = v.clamp(0.0, 1.0);
                }
            }
            "stroke-width" => {
                if let Ok(length) = value.parse::<Length>() {
                    line_weight = (length.number as f32).max(0.0);
                }
            }
            _ => {}
        }
    }

    line_color.a *= global_alpha;
    fill_color.a *= global_alpha;

    BodyMeta {
        id,
        z_index,
        line_weight,
        line_color,
        fill_color,
        smooth_edges: true,
    }
}

fn parse_simple_polygon_path<T: FloatingPointNumber>(d: &str) -> Vec<Vector2<T>> {
    fn to_t<T: FloatingPointNumber>(v: f64) -> T {
        T::from(v as f32)
    }

    let mut current = Vector2::new(T::zero(), T::zero());
    let mut points: Vec<Vector2<T>> = Vec::new();

    for segment in PathParser::from(d) {
        let segment = match segment {
            Ok(segment) => segment,
            Err(_) => return Vec::new(),
        };

        match segment {
            PathSegment::MoveTo { abs, x, y } => {
                current = if abs {
                    Vector2::new(to_t::<T>(x), to_t::<T>(y))
                } else {
                    Vector2::new(current.x + to_t::<T>(x), current.y + to_t::<T>(y))
                };
                points.push(current);
            }
            PathSegment::LineTo { abs, x, y } => {
                current = if abs {
                    Vector2::new(to_t::<T>(x), to_t::<T>(y))
                } else {
                    Vector2::new(current.x + to_t::<T>(x), current.y + to_t::<T>(y))
                };
                points.push(current);
            }
            PathSegment::HorizontalLineTo { abs, x } => {
                current = if abs {
                    Vector2::new(to_t::<T>(x), current.y)
                } else {
                    Vector2::new(current.x + to_t::<T>(x), current.y)
                };
                points.push(current);
            }
            PathSegment::VerticalLineTo { abs, y } => {
                current = if abs {
                    Vector2::new(current.x, to_t::<T>(y))
                } else {
                    Vector2::new(current.x, current.y + to_t::<T>(y))
                };
                points.push(current);
            }
            PathSegment::ClosePath { .. } => {
                break;
            }
            PathSegment::CurveTo { .. }
            | PathSegment::SmoothCurveTo { .. }
            | PathSegment::Quadratic { .. }
            | PathSegment::SmoothQuadratic { .. }
            | PathSegment::EllipticalArc { .. } => {
                return Vec::new();
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

fn bbox_union_recursive<T: FloatingPointNumber>(
    body: &Body<T>,
    min: &mut Vector2<T>,
    max: &mut Vector2<T>,
) {
    for particle in body.particles.iter() {
        *min = min.inf(&particle.position);
        *max = max.sup(&particle.position);
    }

    for child in body.children.iter() {
        bbox_union_recursive(child, min, max);
    }
}

fn normalized_template_transform<T: FloatingPointNumber>(bodies: &[Body<T>]) -> Transformation<T> {
    let mut min = Vector2::new(T::max_value().unwrap(), T::max_value().unwrap());
    let mut max = Vector2::new(T::min_value().unwrap(), T::min_value().unwrap());
    let mut has_points = false;

    for body in bodies.iter() {
        has_points |= !body.particles.is_empty();
        bbox_union_recursive(body, &mut min, &mut max);
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

fn collect_instantiated_meta_recursive<T>(
    template: &Body<T>,
    instance: &Body<T>,
    template_meta: &HashMap<BodyId, BodyMeta>,
    instance_meta: &mut HashMap<BodyId, BodyMeta>,
) {
    if let Some(meta) = template_meta.get(&template.id) {
        let mut copied_meta = meta.clone();
        copied_meta.id = instance.id;
        instance_meta.insert(instance.id, copied_meta);
    }

    for (template_child, instance_child) in template.children.iter().zip(instance.children.iter()) {
        collect_instantiated_meta_recursive(
            template_child,
            instance_child,
            template_meta,
            instance_meta,
        );
    }
}

pub fn instantiate_svg_body<T: FloatingPointNumber>(
    template: &Body<T>,
    template_meta: &HashMap<BodyId, BodyMeta>,
    transformation: Transformation<T>,
) -> (Body<T>, HashMap<BodyId, BodyMeta>) {
    let instance = template.duplicate_with_transformation(transformation);
    let mut instance_meta = HashMap::new();

    collect_instantiated_meta_recursive(template, &instance, template_meta, &mut instance_meta);

    (instance, instance_meta)
}

pub fn instantiate_svg_bodies<T: FloatingPointNumber>(
    templates: &[Body<T>],
    template_meta: &HashMap<BodyId, BodyMeta>,
    transformation: Transformation<T>,
) -> (Vec<Body<T>>, HashMap<BodyId, BodyMeta>) {
    let mut instances = Vec::with_capacity(templates.len());
    let mut instance_meta = HashMap::new();

    for template in templates {
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

    body.constraints.push(Box::new(AreaConstraint::new(
        &body.particles,
        settings.constraint_settings.stiffness_area,
    )));

    add_boundary_bending_constraints(&mut body, settings.constraint_settings.stiffness_bending);
    add_shear_constraints(&mut body, settings.constraint_settings.stiffness_shear);

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
                    parse_svg_path_to_body(path, z_index, anchor, settings)
                {
                    meta_map.insert(body.id, meta);
                    bodies.push(body);
                }
            }
            SvgNode::G(group) => {
                bodies.extend(parse_group_recursive(
                    group,
                    z_index + 1,
                    anchor,
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
    let (direct_paths, child_groups) = split_group_children(group);

    // If a group has no direct path, bubble child group bodies up.
    if direct_paths.is_empty() {
        return child_groups
            .into_iter()
            .flat_map(|child_group| {
                parse_group_recursive(child_group, z_index + 1, anchor, meta_map, settings)
            })
            .collect();
    }

    let mut bodies = Vec::new();
    for path in direct_paths {
        if let Some((mut body, meta, body_anchor)) =
            parse_svg_path_to_body(path, z_index, anchor, settings)
        {
            meta_map.insert(body.id, meta);

            let mut parsed_children = Vec::new();
            for child_group in &child_groups {
                parsed_children.extend(parse_group_recursive(
                    child_group,
                    z_index + 1,
                    Some(body_anchor),
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

fn split_group_children(group: &Group) -> (Vec<&SvgPath>, Vec<&Group>) {
    let mut direct_paths = Vec::new();
    let mut child_groups = Vec::new();

    for node in &group.children {
        match node {
            SvgNode::Path(path) => direct_paths.push(path),
            SvgNode::G(child_group) => child_groups.push(child_group),
            SvgNode::Unknown => {}
        }
    }

    (direct_paths, child_groups)
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
