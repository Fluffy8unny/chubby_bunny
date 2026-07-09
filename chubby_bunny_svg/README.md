# chubby_bunny_svg

SVG parsing and soft-body template generation for Chubby Bunny.

This crate turns polygon-style SVG structures into hierarchical `Body` templates, extracts rendering metadata, and can auto-generate constraints and attachment springs for physics-ready scenes.

## Features

- Parse nested SVG groups and path polygons into `Body` hierarchies
- Preserve style metadata (fill/stroke/z-order) into `MetaMap`
- Auto-generate intrinsic and parent-child attachment constraints
- Instantiate parsed templates with translation, scale, and rotation
- Utility loaders for single-body and multi-body workflows

## Example: load and instantiate an SVG body

This example mirrors the approach used in `examples/svg_example`.

```rust
use chubby_bunny_core::Transformation;
use chubby_bunny_svg::{
    load_svg, instantiate_svg_bodies, BodySettings, SVGConstraintSettings,
};
use nalgebra::Vector2;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svg_source = r#"
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <g>
                <path d="M 10 10 L 90 10 L 90 90 L 10 90 Z" style="fill:#f4d35e;stroke:#000000;stroke-width:2" />
                <g>
                    <path d="M 30 30 L 70 30 L 70 70 L 30 70 Z" style="fill:#ee964b;stroke:#000000;stroke-width:2" />
                </g>
            </g>
        </svg>
    "#;

    let body_settings = BodySettings::from_values(1.0_f32, 0.01, false);
    let constraint_settings = SVGConstraintSettings::from_values(
        0.5, // distance
        0.35, // shear
        0.3, // bending
        0.4, // area
        0.5, // attachment
        5,   // child sample stride
        8,   // max attachments
        2.0, // max distance factor
        3,   // parent springs per anchor
    );

    let (templates, template_meta) = load_svg(
        svg_source,
        &body_settings,
        &constraint_settings,
    )?;

    let transform = Transformation {
        offset: Vector2::new(400.0, 300.0),
        scale: 180.0,
        rotation_radians: 0.0,
    };

    let (instances, instance_meta) = instantiate_svg_bodies(&templates, &template_meta, transform);

    println!("loaded {} body instances", instances.len());
    println!("meta entries: {}", instance_meta.len());
    Ok(())
}
```

## Full scene example

- SVG-driven demo: https://github.com/Fluffy8unny/chubby_bunny/tree/master/examples/svg_example
