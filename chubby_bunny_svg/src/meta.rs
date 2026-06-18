use chubby_bunny_core::BodyId;
use svgtypes::{Paint, PaintFallback};

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
}

impl Color {
    pub fn from_paint(value: &str) -> Option<Self> {
        match Paint::from_str(value).ok()? {
            Paint::None => Some(Self { r: 0, g: 0, b: 0, a: 0.0 }),
            Paint::Color(parsed) => Some(Self {
                r: parsed.red,
                g: parsed.green,
                b: parsed.blue,
                a: (parsed.alpha as f32 / 255.0).clamp(0.0, 1.0),
            }),
            Paint::FuncIRI(_, Some(PaintFallback::None)) => Some(Self { r: 0, g: 0, b: 0, a: 0.0 }),
            Paint::FuncIRI(_, Some(PaintFallback::Color(parsed))) => Some(Self {
                r: parsed.red,
                g: parsed.green,
                b: parsed.blue,
                a: (parsed.alpha as f32 / 255.0).clamp(0.0, 1.0),
            }),
            _ => None,
        }
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
