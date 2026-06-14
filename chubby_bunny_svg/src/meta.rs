use chubby_bunny_core::BodyId;

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BodyMeta {
    pub id: BodyId,
    pub z_index: i32,
    pub line_weight: f32,
    pub line_color: Color,
    pub fill_color: Color,
    pub smooth_edges: bool,
}
