use chubby_bunny_core::{Body, BodyId};
use chubby_bunny_svg::BodyMeta;
use std::collections::HashMap;

#[derive(serde::Serialize)]
pub enum EventType {
    Selection,
    Deselection,
}

#[derive(serde::Serialize)]
pub struct OutgoingEvent {
    pub event_type: EventType,
    pub body_id: BodyId,
    pub description: String,
    pub time_stamp: f32,
}

#[derive(serde::Serialize)]
pub struct PolygonArray {
    pub vertices: Vec<(f32, f32)>,
    pub meta: BodyMeta,
    pub z_index: i32,
    pub children: Vec<PolygonArray>,
}

pub fn default_meta(id: BodyId, z_index: i32) -> BodyMeta {
    BodyMeta {
        id,
        z_index,
        line_weight: 3.0,
        line_color: chubby_bunny_svg::Color {
            r: 183,
            g: 215,
            b: 168,
            a: 1.0,
        },
        fill_color: chubby_bunny_svg::Color {
            r: 143,
            g: 216,
            b: 199,
            a: 0.23,
        },
        smooth_edges: true,
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
