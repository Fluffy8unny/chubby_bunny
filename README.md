![Chubby Bunny logo](readme_assets/logo.png)

# Chubby Bunny

Chubby Bunny is a Rust workspace for WebAssembly-compatible soft-body physics. It lets you design polygonal shapes in a vector editor like Inkscape, feed them through an SVG pipeline that automatically builds hierarchical bodies and constraints, and run the simulation interactively in a browser.


> 🐰 **Live demo:** [chubby bunny example](http://weissenburger.info)

---

## Table of Contents

- [Why use this?](#why-use-this)
- [Design philosophy](#design-philosophy)
- [Available constraints](#available-constraints)
- [SVG pipeline](#svg-pipeline)
- [Examples](#examples)
- [Workspace crates](#workspace-crates)
- [Getting started](#getting-started)
- [Project status](#project-status)

---

## Why use this?

| Feature | What it means for you |
|---|---|
| **SVG-to-body pipeline** | Draw shapes in Inkscape (or any SVG editor) and import them directly — no manual vertex wrangling |
| **Hierarchical body modeling** | Nest bodies inside other bodies to build complex characters from simple parts |
| **Automatic constraint generation** | The pipeline infers distance, area, and attachment constraints from your SVG structure |
| **WASM / browser-first** | The entire simulation runs in the browser via `wasm-bindgen` — no server required at runtime |
| **Tunable soft-body behavior** | Mix stiff and squishy parts by adjusting constraint stiffness on a per-body basis |

---

## Design philosophy

The system is organized around modular bodies arranged in a hierarchy. Constraints describe the relationships between those bodies, while forces act on them to drive motion and interaction. That separation keeps the model composable: bodies define the structure, constraints define how pieces relate, and forces influence the whole system externally.
```mermaid
graph LR
    A[Particles] --> B[Bodies]
    C[Constraints] --> B
    B --> D[Solver]
    M["Metadata(Colors,lineweight)"] --> E
    D --> E[Canvas renderer]
    E --> F[Browser / WASM]
```



---

## Available constraints

Constraints describe the physical properties of bodies. By adding them to a shape you can create stiff or squishy behavior. Constraints can be added manually or generated automatically by the SVG pipeline.

![Constraint gif](readme_assets/constraints.gif)

### Intrinsic constraints

These act within a single body.

- `DistanceConstraint`: preserves the distance between two particles
- `AreaConstraint`: preserves the signed area of a polygonal body
- `BendingConstraint`: preserves the turning angle at a polygon vertex

### Extrinsic constraints

These act between bodies or between a body and an external structure.

- `AttachmentConstraint`: connects child body particles to parent body particles
- `WallConstraint`: keeps bodies on one side of a parent-defined wall segment

### Collision constraints

Handles collision between bodies.

- `CollisionConstraint`: resolves edge intersections and containment contacts between sibling bodies

---

## SVG pipeline

![Workflow example](readme_assets/workflow.png)

The SVG pipeline is designed for polygonal shapes and nested group hierarchies.

Typical flow:

1. Parse an SVG into bodies with metadata.
2. Normalize the result into a unit-space template.
3. Optionally add automatic constraints for the parsed hierarchy.
4. Instantiate the template with transformations when needed.
5. 
```mermaid
graph LR
    SVG[SVG file] --> Parse[Parse shapes & groups]
    Parse --> Normalize[Normalize to unit space]
    Parse --> Metadata[Colors,Lines]-->Template
    Normalize --> Template[Body Template]
    Template --> Constraints[Generate constraints]
    Constraints --> Instantiate[Instantiate with transforms]
```


---

## Examples

The repository includes several WASM examples under `examples/`.

| Example | What it demonstrates |
|---|---|
| `minimal_box` | Minimal setup for a soft-body scene  |
| `contraint_example` | Side-by-side comparison of different constraint configurations |
| `interactive_example` | Interactive selection and dragging |
| `svg_example` | SVG-driven body generation from an imported file |

---

## Workspace crates

- `chubby_bunny_core`: physics primitives — particles, bodies, and constraints
- `chubby_bunny_svg`: SVG parsing, metadata extraction, and automatic constraint generation
- `chubby_bunny_canvas_renderer`: lightweight canvas rendering helpers
- `chubby_bunny_bindgen`: WASM-facing binding helpers
- `chubby_bunny_playground`: playground website featuring a lot of cute bunnies

---

## Getting started

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [`wasm-bindgen-cli`](https://rustwasm.github.io/wasm-bindgen/) — installed automatically by the build scripts if not present
- Python 3 (for the local dev server or whatever you want to use)
- A wasm32 target: `rustup target add wasm32-unknown-unknown`

### 1. Build the Rust workspace

```sh
cargo build
```

### 2. Build all WASM examples at once

The root `build.sh` script builds every example and then starts a local server on port 8000:

```sh
./build.sh
```

To build a single example instead:

```sh
./examples/minimal_box/build.sh
```

### 3. Serve and open in the browser

If you used `./build.sh` the server is already running. Otherwise start it manually:

```sh
python3 -m http.server 8000
```

Then open an example page:

| Example | URL |
|---|---|
| `minimal_box` | <http://localhost:8000/examples/minimal_box/web/> |
| `contraint_example` | <http://localhost:8000/examples/contraint_example/web/> |
| `interactive_example` | <http://localhost:8000/examples/interactive_example/web/> |
| `svg_example` | <http://localhost:8000/examples/svg_example/web/> |

---

## Project status

Chubby Bunny is an active personal project and the API is still evolving. The core simulation, SVG pipeline, and browser demos are working and usable. Breaking changes between versions should be expected until a stable 1.0 is tagged.
