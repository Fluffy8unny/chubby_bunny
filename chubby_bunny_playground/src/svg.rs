use super::js_types::{BodyMeta, Color};
use chubby_bunny::{AreaConstraint, Body, BodyId, DistanceConstraint, Particle};
use nalgebra::Vector2;
use serde::Deserialize;
use std::collections::HashMap;

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
    pub id: Option<String>,
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

fn parse_single(tokens: &[String], i: usize) -> Option<(f32, usize)> {
    let v = tokens.get(i)?.parse::<f32>().ok()?;
    Some((v, i + 1))
}

fn parse_xy_pair(tokens: &[String], i: usize) -> Option<(Vector2<f32>, usize)> {
    let x = tokens.get(i)?.parse::<f32>().ok()?;
    let y = tokens.get(i + 1)?.parse::<f32>().ok()?;
    Some((Vector2::new(x, y), i + 2))
}

fn parse_simple_polygon_path(d: &str) -> Vec<Vector2<f32>> {
    let tokens = tokenize_path_data(d);
    let mut i = 0usize;
    let mut command = 'M';
    let mut current = Vector2::new(0.0f32, 0.0f32);
    let mut points: Vec<Vector2<f32>> = Vec::new();

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
                if let Some((p, next_i)) = parse_xy_pair(&tokens, i) {
                    current = p;
                    points.push(current);
                    i = next_i;
                    command = 'L';
                } else {
                    break;
                }
            }
            'm' => {
                if let Some((p, next_i)) = parse_xy_pair(&tokens, i) {
                    current = Vector2::new(current.x + p.x, current.y + p.y);
                    points.push(current);
                    i = next_i;
                    command = 'l';
                } else {
                    break;
                }
            }
            'L' => {
                if let Some((p, next_i)) = parse_xy_pair(&tokens, i) {
                    current = p;
                    points.push(current);
                    i = next_i;
                } else {
                    break;
                }
            }
            'l' => {
                if let Some((p, next_i)) = parse_xy_pair(&tokens, i) {
                    current = Vector2::new(current.x + p.x, current.y + p.y);
                    points.push(current);
                    i = next_i;
                } else {
                    break;
                }
            }
            'H' => {
                if let Some((x, next_i)) = parse_single(&tokens, i) {
                    current = Vector2::new(x, current.y);
                    points.push(current);
                    i = next_i;
                } else {
                    break;
                }
            }
            'h' => {
                if let Some((x, next_i)) = parse_single(&tokens, i) {
                    current = Vector2::new(current.x + x, current.y);
                    points.push(current);
                    i = next_i;
                } else {
                    break;
                }
            }
            'V' => {
                if let Some((y, next_i)) = parse_single(&tokens, i) {
                    current = Vector2::new(current.x, y);
                    points.push(current);
                    i = next_i;
                } else {
                    break;
                }
            }
            'v' => {
                if let Some((y, next_i)) = parse_single(&tokens, i) {
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

fn bbox_origin(points: &[Vector2<f32>]) -> Vector2<f32> {
    points
        .iter()
        .fold(Vector2::new(f32::INFINITY, f32::INFINITY), |acc, p| {
            Vector2::new(acc.x.min(p.x), acc.y.min(p.y))
        })
}

fn normalize_points_to_anchor(
    points: Vec<Vector2<f32>>,
    anchor: Vector2<f32>,
) -> Vec<Vector2<f32>> {
    points
        .into_iter()
        .map(|p| Vector2::new(p.x - anchor.x, p.y - anchor.y))
        .collect()
}

fn parse_svg_path_to_body(
    path: &SvgPath,
    z_index: i32,
    anchor: Option<Vector2<f32>>,
) -> Option<(Body, BodyMeta, Vector2<f32>)> {
    let d = path.d.as_deref()?;
    let points = parse_simple_polygon_path(d);
    if points.len() < 3 {
        return None;
    }
    let anchor = anchor.unwrap_or_else(|| bbox_origin(&points));
    let points = normalize_points_to_anchor(points, anchor.clone());

    let mut body = Body::empty();
    for point in points {
        body.particles
            .push(Particle::new(point, Vector2::zeros(), 1.0, 0.01, false));
    }

    for i in 0..body.particles.len() {
        body.constraints.push(Box::new(DistanceConstraint::new(
            i,
            (i + 1) % body.particles.len(),
            &body.particles,
            1.0,
        )));
    }

    let idxs: Vec<usize> = (0..body.particles.len()).collect();
    body.constraints
        .push(Box::new(AreaConstraint::new(idxs, &body.particles, 1.0)));

    let style = path.style.as_deref().unwrap_or("");
    let meta = parse_style_to_body_meta(style, body.id, z_index);

    Some((body, meta, anchor))
}

fn parse_svg_to_hierarchy(svg: &Svg) -> (Vec<Body>, HashMap<BodyId, BodyMeta>) {
    let mut meta_map = HashMap::new();
    let bodies = parse_nodes_recursive(&svg.children, 0, None, &mut meta_map);
    (bodies, meta_map)
}

fn parse_nodes_recursive(
    nodes: &[SvgNode],
    z_index: i32,
    anchor: Option<Vector2<f32>>,
    meta_map: &mut HashMap<BodyId, BodyMeta>,
) -> Vec<Body> {
    let mut bodies = Vec::new();
    for node in nodes {
        match node {
            SvgNode::Path(path) => {
                if let Some((body, meta, _anchor)) =
                    parse_svg_path_to_body(path, z_index, anchor.clone())
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
                ));
            }
            SvgNode::Unknown => {}
        }
    }
    bodies
}

fn parse_group_recursive(
    group: &Group,
    z_index: i32,
    anchor: Option<Vector2<f32>>,
    meta_map: &mut HashMap<BodyId, BodyMeta>,
) -> Vec<Body> {
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
                parse_group_recursive(child_group, z_index + 1, anchor.clone(), meta_map)
            })
            .collect();
    }

    let mut bodies = Vec::new();
    for path in direct_paths {
        if let Some((mut body, meta, body_anchor)) =
            parse_svg_path_to_body(path, z_index, anchor.clone())
        {
            meta_map.insert(body.id, meta);

            for child_group in &child_groups {
                let children = parse_group_recursive(
                    child_group,
                    z_index + 1,
                    Some(body_anchor.clone()),
                    meta_map,
                );
                body.children.extend(children);
            }

            bodies.push(body);
        }
    }

    bodies
}

pub fn load_svg(xml: &str) -> (Vec<Body>, HashMap<BodyId, BodyMeta>) {
    let svg = quick_xml::de::from_str(xml)
        .expect("Failed to parse SVG XML. Ensure the input is a valid SVG string.");
    parse_svg_to_hierarchy(&svg)
}
