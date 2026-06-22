![Chubby Bunny logo](readme_assets/logo.png)

# Chubby Bunny

Chubby Bunny is a Rust workspace for wasm-compatible soft-body physics.

It combines three main pieces:

- a soft-body physics core built around particles and constraints
- an SVG pipeline that can turn polygonal SVG shapes into bodies and automatically generate useful constraints
- a simple canvas renderer and wasm bindings for interactive browser demos

This makes it easy to design shapes in programs like inkscape and use them inside of a browser envoirement.
You can check out an example at [chubby bunny example](http://weissenburger.info).
## Table of Contents

- [Design Philosophy](#design-philosophy)
- [Available Constraints](#available-constraints)
- [SVG Pipeline](#svg-pipeline)
- [Examples](#examples)
- [Workspace Crates](#workspace-crates)
- [Getting Started](#getting-started)
- [Project Status](#project-status)

## Design Philosophy

The system is organized around modular bodies arranged in a hierarchy. Constraints describe the relationships between those bodies, while forces act on them to drive motion and interaction. That separation keeps the model composable: bodies define the structure, constraints define how pieces relate, and forces define how the system evolves.

## Available Constraints

Constraints describe the physical properties of bodies. By adding them to a shape, you can create stiff or squishy behaivor. These can either be added manually or automatically.

![Constraint gif](readme_assets/constraints.gif)
### Intrinsic Constraints
These act within a single body.

- `DistanceConstraint`: preserves the distance between two particles
- `AreaConstraint`: preserves the signed area of a polygonal body
- `BendingConstraint`: preserves the turning angle at a polygon vertex

### Extrinsic Constraints

These act between bodies or between a body and an external structure.

- `AttachmentConstraint`: connects child body particles to parent body particles
- `WallConstraint`: keeps bodies on one side of a parent-defined wall segment

### Collision Constraints

Handles colision between bodies.

- `CollisionConstraint`: resolves edge intersections and containment contacts between sibling bodies


## SVG Pipeline
![Worklflow example](readme_assets/workflow.png)
The SVG pipeline is designed for polygonal shapes and nested group hierarchies.

Typical flow:

1. Parse an SVG into bodies with metadata.
2. Normalize the result into a unit-space template.
3. Optionally add automatic constraints for the parsed hierarchy.
4. Instantiate the template with transformations when needed.


## Examples

The repository includes several wasm examples under `examples/`.

- `minimal_box`: minimal setup for a soft-body scene
- `contraint_example`: constraint-focused demo crate in the workspace
- `interactive_example`: interactive selection and dragging demo
- `svg_example`: SVG-driven body generation demo

See [examples/README.md](examples/README.md) for the current examples workflow.

## Workspace Crates

- `chubby_bunny_core`: physics primitives, particles, bodies, and constraints
- `chubby_bunny_svg`: SVG parsing, metadata extraction, and automatic constraint generation
- `chubby_bunny_canvas_renderer`: lightweight canvas rendering helpers
- `chubby_bunny_bindgen`: wasm-facing binding helpers
- `chubby_bunny_playground`: Code for a website, that contains a lot of cute bunnies

## Getting Started

Build the workspace:

```sh
cargo build
```

Build an example:

```sh
./examples/minimal_box/build.sh
```

Serve the repository locally:

```sh
python3 -m http.server 8000
```

Then open an example page such as:

```text
http://localhost:8000/examples/minimal_box/web/
```