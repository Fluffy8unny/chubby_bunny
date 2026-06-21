![Chubby Bunny logo](web/assets/logo.svg)

# Chubby Bunny

Chubby Bunny is a Rust workspace for wasm-compatible soft-body physics.

It combines three main pieces:

- a soft-body physics core built around particles and constraints
- an SVG pipeline that can turn polygonal SVG shapes into bodies and automatically generate useful constraints
- a simple canvas renderer and wasm bindings for interactive browser demos

The goal is to make it easy to prototype deformable 2D bodies in Rust and run them in the browser with a minimal integration layer.

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Available Constraints](#available-constraints)
- [SVG Pipeline](#svg-pipeline)
- [Examples](#examples)
- [Workspace Crates](#workspace-crates)
- [Getting Started](#getting-started)
- [Project Status](#project-status)

## Overview

Chubby Bunny is aimed at lightweight 2D soft-body experiments that work well with WebAssembly. The workspace is split into focused crates so you can use only the parts you need.

At a high level, you can:

- build bodies directly from particles and constraints
- generate bodies automatically from SVG polygons
- attach nested SVG shapes to parent bodies through attachment constraints
- render the result in a browser using the included canvas renderer
- expose demos or applications through the wasm bindings layer

## Features

- Wasm-compatible soft-body physics written in Rust
- Particle-based bodies with intrinsic, extrinsic, and collision constraints
- Automatic body generation from polygonal SVG paths
- Automatic SVG constraint generation for outline, area, shear, bending, and attachments
- Simple HTML canvas renderer for browser demos
- Small example applications for testing and iteration

## Available Constraints

The project currently exposes and uses the following constraint types.

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

These resolve interactions between bodies.

- `CollisionConstraint`: resolves edge intersections and containment contacts between sibling bodies

### SVG-Generated Constraints

When using the SVG utilities, the following helpers can be added automatically:

- boundary distance constraints
- shear constraints
- boundary bending constraints
- area constraints
- parent-child attachment constraints

## SVG Pipeline

The SVG pipeline is designed for polygonal shapes and nested group hierarchies.

Typical flow:

1. Parse an SVG into bodies with metadata.
2. Normalize the result into a unit-space template.
3. Optionally add automatic constraints for the parsed hierarchy.
4. Instantiate the template with transformations when needed.

This gives you two modes of use:

- automatic setup for fast iteration
- manual setup when you want full control over generated constraints

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
- `chubby_bunny_playground`: higher-level playground integration

## Getting Started

Build the workspace:

```sh
cargo build
```

Run checks:

```sh
cargo check
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

## Project Status

This README is a starting point and will expand with API examples, SVG authoring guidance, and more detailed setup notes.
