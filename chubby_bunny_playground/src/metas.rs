use chubby_bunny_core::BodyId;
use chubby_bunny_svg::BodyMeta;

pub fn default_meta_for_container(id: BodyId) -> BodyMeta {
    BodyMeta {
        id,
        z_index: 0,
        line_weight: 0.0,
        line_color: chubby_bunny_svg::Color {
            r: 250,
            g: 246,
            b: 240,
            a: 0.0,
        },
        fill_color: chubby_bunny_svg::Color {
            r: 250,
            g: 246,
            b: 240,
            a: 0.0,
        },
        smooth_edges: false,
    }
}

pub fn selected_meta(id: BodyId, z_index: i32) -> BodyMeta {
    BodyMeta {
        id,
        z_index,
        line_weight: 3.0,
        line_color: chubby_bunny_svg::Color {
            r: 200,
            g: 152,
            b: 108,
            a: 1.0,
        },
        fill_color: chubby_bunny_svg::Color {
            r: 200,
            g: 152,
            b: 108,
            a: 0.5,
        },
        smooth_edges: true,
    }
}
