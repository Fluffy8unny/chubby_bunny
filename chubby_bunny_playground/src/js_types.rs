use crate::{Body, BodyId};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
}

#[wasm_bindgen]
#[derive(Debug, Clone, serde::Serialize)]
pub struct BodyMeta {
    pub id: BodyId,
    pub z_index: i32,
    pub line_weight: f32,
    pub line_color: Color,
    pub fill_color: Color,
}

#[derive(serde::Serialize)]
pub struct PolygonArray {
    vertices: Vec<(f32, f32)>,
    meta: BodyMeta,
    z_index: i32,
    children: Vec<PolygonArray>,
}

fn default_meta(id: BodyId, z_index: i32) -> BodyMeta {
    BodyMeta {
        id,
        z_index,
        line_weight: 3.0,
        line_color: Color {
            r: 183,
            g: 215,
            b: 168,
            a: 1.0,
        },
        fill_color: Color {
            r: 143,
            g: 216,
            b: 199,
            a: 0.23,
        },
    }
}

pub fn body_to_polygon_array(
    body: &Body,
    meta_data: &HashMap<BodyId, BodyMeta>,
    depth: i32,
) -> PolygonArray {
    let vertices: Vec<(f32, f32)> = body
        .particles
        .iter()
        .map(|p| (p.position.x, p.position.y))
        .collect();
    let children = body
        .children
        .iter()
        .map(|child| body_to_polygon_array(child, meta_data, depth + 1))
        .collect();
    let meta = meta_data
        .get(&body.id)
        .cloned()
        .unwrap_or_else(|| default_meta(body.id, depth));

    PolygonArray {
        vertices,
        meta,
        z_index: depth,
        children,
    }
}

pub fn bodies_to_polygon_arrays<'a, I>(
    bodies: I,
    meta_data: &HashMap<BodyId, BodyMeta>,
) -> Vec<PolygonArray>
where
    I: IntoIterator<Item = &'a Body>,
{
    bodies
        .into_iter()
        .map(|body| body_to_polygon_array(body, meta_data, 0))
        .collect()
}
