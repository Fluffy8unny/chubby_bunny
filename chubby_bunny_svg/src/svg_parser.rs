use crate::meta::{parse_style_to_body_meta, BodyMeta, MetaMap};
use crate::settings::{BodySettings, SVGConstraintSettings};
use crate::svg_constraints::{
    add_area_constraints, add_boundary_bending_constraints, add_boundary_distance_constraints,
    add_shear_constraints, attach_child_to_parent,
};
use chubby_bunny_core::{Body, FloatingPointNumber, Particle, Transformation};

use nalgebra::Vector2;
use serde::Deserialize;
use std::collections::HashMap;
use svgtypes::{PathParser, PathSegment};

pub type SVGLoadResult<T> = Result<(Vec<Body<T>>, MetaMap), Box<dyn std::error::Error>>;
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
    let Some((first, rest)) = points.split_first() else {
        return Vector2::zeros();
    };

    rest.iter().fold(*first, |acc, point| {
        Vector2::new(acc.x.min(point.x), acc.y.min(point.y))
    })
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

fn collect_instantiated_meta_recursive<T>(
    template: &Body<T>,
    instance: &Body<T>,
    template_meta: &MetaMap,
    instance_meta: &mut MetaMap,
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
    template_meta: &MetaMap,
    transformation: Transformation<T>,
) -> (Body<T>, MetaMap) {
    let mut instance = template.clone();
    let mut instance_meta = MetaMap::new();

    instance.transform(transformation);
    collect_instantiated_meta_recursive(template, &instance, template_meta, &mut instance_meta);
    (instance, instance_meta)
}

pub fn instantiate_svg_bodies<T: FloatingPointNumber>(
    templates: &[Body<T>],
    template_meta: &MetaMap,
    transformation: Transformation<T>,
) -> (Vec<Body<T>>, MetaMap) {
    let mut instances = Vec::with_capacity(templates.len());
    let mut instance_meta = MetaMap::new();

    for template in templates {
        let (instance, meta) = instantiate_svg_body(template, template_meta, transformation);
        instances.push(instance);
        instance_meta.extend(meta);
    }
    (instances, instance_meta)
}

fn svg_to_body_instance<T: FloatingPointNumber>(
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

    let style = path.style.as_deref().unwrap_or("");
    let meta = parse_style_to_body_meta(style, body.id, z_index);

    Some((body, meta, anchor))
}

fn parse_nodes_recursive<T: FloatingPointNumber>(
    nodes: &[SvgNode],
    z_index: i32,
    anchor: Option<Vector2<T>>,
    meta_map: &mut MetaMap,
    settings: &BodySettings<T>,
) -> Vec<Body<T>> {
    let mut bodies = Vec::new();
    for node in nodes {
        match node {
            SvgNode::Path(path) => {
                if let Some((body, meta, _anchor)) =
                    svg_to_body_instance(path, z_index, anchor, settings)
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
    meta_map: &mut MetaMap,
    settings: &BodySettings<T>,
) -> Vec<Body<T>> {
    let (svg_paths, child_groups) = split_paths_and_groups(group);

    // If a group has no path, bubble child group bodies up.
    if svg_paths.is_empty() {
        return child_groups
            .into_iter()
            .flat_map(|child_group| {
                parse_group_recursive(child_group, z_index + 1, anchor, meta_map, settings)
            })
            .collect();
    }

    let mut bodies = Vec::new();
    for path in svg_paths {
        if let Some((mut body, meta, body_anchor)) =
            svg_to_body_instance(path, z_index, anchor, settings)
        {
            meta_map.insert(body.id, meta);

            let parsed_children = child_groups
                .iter()
                .flat_map(|child_group| {
                    parse_group_recursive(
                        child_group,
                        z_index + 1,
                        Some(body_anchor),
                        meta_map,
                        settings,
                    )
                })
                .collect::<Vec<_>>();

            for child in parsed_children {
                body.children.push(child);
            }
            bodies.push(body);
        }
    }
    bodies
}

fn split_paths_and_groups(group: &Group) -> (Vec<&SvgPath>, Vec<&Group>) {
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

pub fn load_svg_to_body<T: FloatingPointNumber>(
    xml: &str,
    body_settings: &BodySettings<T>,
) -> SVGLoadResult<T> {
    let svg: Svg = quick_xml::de::from_str(xml)?;
    let mut template_meta = HashMap::new();
    let mut templates =
        parse_nodes_recursive(&svg.children, 0, None, &mut template_meta, body_settings);
    let normalization = normalized_template_transform(&templates);
    templates
        .iter_mut()
        .for_each(|template| template.transform(normalization));
    Ok((templates, template_meta))
}

fn add_automatic_constraints_recursive<T: FloatingPointNumber>(
    body: &mut Body<T>,
    constraint_settings: &SVGConstraintSettings<T>,
) {
    body.constraints.clear();
    body.children_constraints.clear();

    add_boundary_distance_constraints(
        body,
        constraint_settings.constraint_settings.stiffness_distance,
    );
    add_area_constraints(body, constraint_settings.constraint_settings.stiffness_area);
    add_boundary_bending_constraints(
        body,
        constraint_settings.constraint_settings.stiffness_bending,
    );
    add_shear_constraints(
        body,
        constraint_settings.constraint_settings.stiffness_shear,
    );

    let mut children = std::mem::take(&mut body.children);
    for child in children.iter_mut() {
        add_automatic_constraints_recursive(child, constraint_settings);
    }

    for child in children.iter() {
        attach_child_to_parent(body, child, constraint_settings);
    }

    body.children = children;
}

pub fn add_automatic_constraints<T: FloatingPointNumber>(
    bodies: &mut [Body<T>],
    constraint_settings: &SVGConstraintSettings<T>,
) {
    for body in bodies.iter_mut() {
        add_automatic_constraints_recursive(body, constraint_settings);
    }
}

pub fn load_svg<T: FloatingPointNumber>(
    xml: &str,
    body_settings: &BodySettings<T>,
    constraint_settings: &SVGConstraintSettings<T>,
) -> SVGLoadResult<T> {
    let (mut templates, template_meta) = load_svg_to_body(xml, body_settings)?;
    add_automatic_constraints(&mut templates, constraint_settings);
    Ok((templates, template_meta))
}
