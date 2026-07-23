use crate::{Body, FloatingPointNumber, Force};

/// Advances the simulation with a fixed timestep, running as many equal-sized substeps
/// per frame as the elapsed real time requires.
///
/// The substep *size* is constant, which keeps constraint behavior stable and independent
/// of the render frame rate, while the substep *count* adapts to the elapsed time so the
/// simulation advances at real-time speed. Leftover real time is carried over between frames
/// in an accumulator.
pub struct FixedStepper {
    /// Size of a single simulation substep in seconds.
    fixed_dt: f32,
    /// Upper bound on substeps
    max_substeps: usize,
    /// Number of XPBD solver passes run within each substep. More passes let corrections propagate
    /// through coupled constraints, converging each body toward its compliant solution (stiffer feel).
    iterations: usize,
    /// Unconsumed real time carried over between frames.
    accumulator: f32,
}

impl FixedStepper {
    /// Creates a stepper
    pub fn new(fixed_dt: f32, max_substeps: usize, iterations: usize) -> Self {
        assert!(fixed_dt > 0.0, "fixed_dt must be strictly positive");
        assert!(max_substeps > 0, "max_substeps must be at least one");
        assert!(iterations > 0, "iterations must be at least one");
        Self {
            fixed_dt,
            max_substeps,
            iterations,
            accumulator: 0.0,
        }
    }

    /// Advances every body by `frame_dt` seconds of real time using fixed-size substeps,
    /// applying `forces` uniformly on each substep.
    pub fn advance<T, F>(&mut self, bodies: &mut [Body<T>], forces: &[F], frame_dt: f32)
    where
        T: FloatingPointNumber,
        F: Force<T>,
    {
        let max_frame_dt = self.fixed_dt * self.max_substeps as f32;
        self.accumulator = (self.accumulator + frame_dt.max(0.0)).min(max_frame_dt);

        let step_dt = T::from(self.fixed_dt);
        while self.accumulator >= self.fixed_dt {
            for body in bodies.iter_mut() {
                body.perform_step(forces, step_dt, self.iterations);
            }
            self.accumulator -= self.fixed_dt;
        }
    }
}

impl Default for FixedStepper {
    fn default() -> Self {
        Self::new(1.0 / 360.0, 90, 4)
    }
}
