use chubby_bunny_core::BodyId;
use svgtypes::{Paint, PaintFallback, Length};

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

pub fn parse_style_to_body_meta(style: &str, id: BodyId, z_index: i32) -> BodyMeta {
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
