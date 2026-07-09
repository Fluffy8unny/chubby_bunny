# chubby_bunny_bindgen

Procedural macro helpers for exposing Chubby Bunny game loops to JavaScript via wasm-bindgen.

This crate provides the `#[chubby_bunny_bindgen]` attribute macro, which generates a browser-facing WASM API that wraps your `GameLoop<YourGame>`.

## Features

- Derives a wasm-bindgen friendly API from a Game struct
- Generates constructor and lifecycle methods (`new`, `init`, `update`, `reset`)
- Exposes rendering payload getter (`get_polygon_arrays`)
- Exposes mouse input methods (`mouse_down`, `mouse_move`, `mouse_up`)


## Example: expose a game loop to JS

This is the same integration style used in `examples/minimal_box` and `examples/svg_example`.

```rust
use chubby_bunny_bindgen::chubby_bunny_bindgen;
use chubby_bunny_canvas_renderer::game_loop::{Game, GameLoop};
use chubby_bunny_canvas_renderer::input::Event;
use chubby_bunny_canvas_renderer::js_types::OutgoingEvent;
use chubby_bunny_core::Body;
use chubby_bunny_svg::MetaMap;
use std::collections::VecDeque;

struct MyGame;

impl MyGame {
    fn new() -> Self {
        Self
    }
}

impl Game for MyGame {
    fn init(&mut self, _width: usize, _height: usize) {}

    fn reset(&mut self, _width: f32, _height: f32) {}

    fn update(&mut self, _incoming_events: VecDeque<Event>, _dt_ms: f32) -> Vec<OutgoingEvent> {
        Vec::new()
    }

    fn bodies_to_render(&self) -> &[Body] {
        &[]
    }

    fn meta_data_to_render(&self) -> &MetaMap {
        static EMPTY: std::sync::LazyLock<MetaMap> = std::sync::LazyLock::new(MetaMap::new);
        &EMPTY
    }
}

#[chubby_bunny_bindgen]
pub struct MyWasmGame(GameLoop<MyGame>);
```

After macro expansion, `MyWasmGame` gets a wasm-bindgen compatible API with the following methods:

- `new()`
- `init(width, height)`
- `update(dt_ms)`
- `reset(width, height)`
- `get_polygon_arrays()`
- mouse input handlers

## Full examples

- Minimal game wrapper: https://github.com/Fluffy8unny/chubby_bunny/tree/master/examples/minimal_box
- SVG-based game wrapper: https://github.com/Fluffy8unny/chubby_bunny/tree/master/examples/svg_example
