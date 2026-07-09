# chubby_bunny_canvas_renderer
Browser-facing game loop and rendering helpers for Chubby Bunny scenes.

This crate bridges simulation data (`Body` + metadata) to canvas-ready polygon arrays and provides an input-aware `GameLoop` abstraction.

## Features

- `Game` trait and `GameLoop` driver for frame-based updates
- Mouse input event queue (`mouse_down`, `mouse_move`, `mouse_up`)
- Conversion from bodies and metadata to polygon arrays for JS rendering
- WASM-friendly serialization via `wasm-bindgen` and `serde-wasm-bindgen`

## Example: run a simple loop and render-ready output

This is a reduced form of the pattern used in `examples/minimal_box`.

```rust
use chubby_bunny_canvas_renderer::game_loop::{Game, GameLoop};
use chubby_bunny_canvas_renderer::input::Event;
use chubby_bunny_canvas_renderer::js_types::{default_meta, OutgoingEvent};
use chubby_bunny_canvas_renderer::primitives::{create_polygon, SimpleBodySettings};
use chubby_bunny_core::{force::constant_force, Body, SolverSettings};
use chubby_bunny_svg::MetaMap;
use nalgebra::Vector2;
use std::collections::VecDeque;

struct DemoGame {
    bodies: Vec<Body>,
    meta: MetaMap,
}

impl DemoGame {
    fn new() -> Self {
        Self { bodies: Vec::new(), meta: MetaMap::new() }
    }
}

impl Game for DemoGame {
    fn init(&mut self, width: usize, height: usize) {
        self.bodies.clear();
        self.meta.clear();

        let settings = SimpleBodySettings {
            stiffness_distance: 0.6,
            stiffness_shear: 0.2,
            stiffness_area: 0.4,
            stiffness_bending: 0.2,
            friction: 0.01,
        };

        let body = create_polygon(
            Vector2::new(width as f32 * 0.5, height as f32 * 0.3),
            40.0,
            10,
            &settings,
        );

        self.meta.insert(body.id, default_meta(body.id, 0));
        self.bodies.push(body);
    }

    fn reset(&mut self, width: f32, height: f32) {
        self.init(width as usize, height as usize);
    }

    fn update(&mut self, _incoming_events: VecDeque<Event>, dt_ms: f32) -> Vec<OutgoingEvent> {
        let solver = SolverSettings {
            reference_dt: 1.0 / 60.0,
            constraint_iterations: 6,
        };
        let dt = dt_ms / 1000.0;
        for body in &mut self.bodies {
            let gravity = constant_force(Vector2::new(0.0, 250.0));
            body.perform_step(&[gravity], dt, &solver);
        }
        Vec::new()
    }

    fn bodies_to_render(&self) -> &[Body] {
        &self.bodies
    }

    fn meta_data_to_render(&self) -> &MetaMap {
        &self.meta
    }
}

fn main() {
    let mut loop_state = GameLoop {
        game_impl: Box::new(DemoGame::new()),
        polygon_arrays: Vec::new(),
        user_input: Default::default(),
    };

    loop_state.init(800, 600);
    let _ = loop_state.update(16.0);
    let polygons = loop_state.get_polygon_arrays().unwrap();
    println!("render payload: {:?}", polygons);
}
```

## Full demos

- Minimal scene: https://github.com/Fluffy8unny/chubby_bunny/tree/master/examples/minimal_box
- Interactive input demo: https://github.com/Fluffy8unny/chubby_bunny/tree/master/examples/interactive_example
