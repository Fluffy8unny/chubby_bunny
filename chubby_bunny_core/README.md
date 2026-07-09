# chubby_bunny_core

Core soft-body physics primitives used by the Chubby Bunny ecosystem.

`chubby_bunny_core` is a hierarchical, position-correction soft-body solver.
Each `Body` owns particles and constraints, can contain child bodies, and can add parent-child constraints plus optional child-child collisions.

# How the engine advances one frame

Calling `body.perform_step(forces, dt, solver_settings)` runs this exact order every frame:

1. Force pass (recursive through body tree)
For each non-pinned particle:

$$
a = \frac{F}{m},\quad v_{tmp} = v + a\,dt,\quad p \leftarrow p + v_{tmp}\,dt
$$

2. Constraint pass (`constraint_iterations` times)

- Solve intrinsic constraints per body
- Solve parent-child constraints 
- Solve sibling collisions 


$$
\alpha = \mathrm{clamp}\left(\frac{stiffness\cdot dt}{reference\_dt\cdot iterations},\ 0,\ 1\right)
$$

3. Post integration update (recursive)
After all corrections, velocity is reconstructed and damped:

$$
decay = 1 - friction\cdot\frac{dt}{reference\_dt}
$$

$$
v \leftarrow \frac{p - p_{prev}}{dt}\cdot decay,\quad p_{prev} \leftarrow p
$$

Practical tuning:

- More `constraint_iterations` => stiffer and more stable shapes, higher CPU cost
- Higher `stiffness` => stronger shape preservation
- Higher `friction` => faster damping

## Features

- Hierarchical `Body` model with nested child bodies
- Intrinsic constraints: distance, area, and bending
- Extrinsic constraints: attachment and wall constraints
- Optional sibling collision constraint support
- Generic numeric support through nalgebra-compatible real types

## Example: simulate a soft body step

This example is adapted from the patterns used in `examples/minimal_box` and `examples/interactive_example` in the repository.

```rust
use chubby_bunny_core::{
    force::constant_force, Body, DistanceConstraint, Particle, SolverSettings,
};
use nalgebra::Vector2;

fn make_triangle() -> Body<f32> {
    let mut body = Body::empty();

    body.particles.push(Particle::new(Vector2::new(0.0, 0.0), Vector2::zeros(), 1.0, 0.01, false));
    body.particles.push(Particle::new(Vector2::new(1.0, 0.0), Vector2::zeros(), 1.0, 0.01, false));
    body.particles.push(Particle::new(Vector2::new(0.5, 0.8), Vector2::zeros(), 1.0, 0.01, false));

    // Keep edge lengths stable while gravity acts on the particles.
    for (a, b) in [(0, 1), (1, 2), (2, 0)] {
        body.constraints.push(Box::new(DistanceConstraint::new(
            a,
            b,
            &body.particles,
            0.9,
        )));
    }

    body
}

fn main() {
    let mut body = make_triangle();
    let solver = SolverSettings {
        reference_dt: 1.0 / 60.0,
        constraint_iterations: 6,
    };

    let dt = 1.0 / 60.0;
    for _ in 0..120 {
        let gravity = constant_force(Vector2::new(0.0, 9.81));
        body.perform_step(&[gravity], dt, &solver);
    }

    println!("centroid: {:?}", body.centroid());
}
```

## More complete demos

- Minimal scene: https://github.com/Fluffy8unny/chubby_bunny/tree/master/examples/minimal_box
- Interactive drag/select: https://github.com/Fluffy8unny/chubby_bunny/tree/master/examples/interactive_example
