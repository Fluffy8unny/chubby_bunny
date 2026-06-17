use chubby_bunny_core::BodyId;

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
}

impl Color {
    pub fn set_rgb(&mut self, (r, g, b): (u8, u8, u8)) {
        self.r = r;
        self.g = g;
        self.b = b;
    }

    pub fn black() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 1.0,
        }
    }
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
