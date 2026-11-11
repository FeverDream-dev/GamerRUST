//! Frame timing and the fixed-timestep accumulator.
//!
//! [`Time`] is the per-frame snapshot (delta, elapsed, fps) the engine hands to
//! [`crate::Game::update`]/[`crate::Game::draw`]. [`FixedStep`] is the pure
//! accumulator that decides how many fixed updates are due for a frame — kept
//! free of any windowing/GPU concerns so it can be unit tested headlessly.

/// Per-frame timing snapshot.
#[derive(Clone, Copy, Debug, Default)]
pub struct Time {
    /// Seconds since the previous frame (clamped, never negative).
    pub delta: f32,
    /// Seconds since startup.
    pub elapsed: f32,
    /// The fixed simulation step in seconds (1/60 by default).
    pub fixed_delta: f32,
    /// Smoothed frames-per-second estimate.
    pub fps: f32,
}

impl Time {
    /// A `Time` configured for the given fixed step (Hz). FPS starts at 0.
    pub fn with_fixed_hz(hz: f32) -> Self {
        Self { fixed_delta: 1.0 / hz, ..Default::default() }
    }
}

/// Pure fixed-timestep accumulator: feed it frame `dt`, get how many fixed
/// steps are due. Render interpolation factor comes from [`FixedStep::alpha`].
///
/// ```
/// use gamerrust::time::FixedStep;
/// let mut fs = FixedStep::new(1.0 / 60.0);
/// assert_eq!(fs.tick(1.0 / 60.0), 1);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FixedStep {
    accumulator: f32,
    /// The fixed step in seconds.
    pub step: f32,
}

impl FixedStep {
    // ponytail: catch-up cap 5 steps, make configurable when someone ships a loading stutter
    const MAX_STEPS: usize = 5;

    pub fn new(step: f32) -> Self {
        Self { accumulator: 0.0, step }
    }

    /// Feed a frame delta; returns the number of fixed steps due this frame.
    /// Accumulates leftovers for fractional steps.
    pub fn tick(&mut self, dt: f32) -> usize {
        self.accumulator += dt;
        // Subtraction loop (not division): immune to f32 division drift.
        let mut steps = 0;
        while steps < Self::MAX_STEPS && self.accumulator >= self.step {
            self.accumulator -= self.step;
            steps += 1;
        }
        if self.accumulator >= self.step {
            // Debt beyond the cap is dropped (spiral-of-death guard): a stall
            // must not turn every later frame into another full burst.
            self.accumulator = 0.0;
        }
        steps
    }

    /// Interpolation factor in `[0, 1)`: how far between fixed steps the
    /// current frame is. Use to blend previous/current state when rendering.
    pub fn alpha(&self) -> f32 {
        (self.accumulator / self.step).clamp(0.0, 0.999_999)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEP: f32 = 1.0 / 60.0;

    #[test]
    fn one_step_per_frame_at_60fps() {
        let mut fs = FixedStep::new(STEP);
        for frame in 0..240 {
            assert_eq!(fs.tick(STEP), 1, "frame {frame} should owe exactly one step");
        }
    }

    #[test]
    fn no_step_due_when_no_time_passed() {
        let mut fs = FixedStep::new(STEP);
        assert_eq!(fs.tick(0.0), 0);
        assert_eq!(fs.tick(STEP * 0.5), 0);
        assert!((fs.alpha() - 0.5).abs() < 1e-6, "half a step banked");
        assert_eq!(fs.tick(STEP * 0.5), 1, "the two halves add up to one step");
        assert_eq!(fs.alpha(), 0.0);
    }

    #[test]
    fn fractional_accumulation_across_uneven_frames() {
        // 90 fps frames (two thirds of a step each): no time may be lost,
        // emitted steps + banked remainder must equal the time fed in.
        let mut fs = FixedStep::new(STEP);
        let dt = STEP * 2.0 / 3.0;
        let mut steps = 0usize;
        let mut fed = 0.0f32;
        for _ in 0..6 {
            steps += fs.tick(dt);
            fed += dt;
        }
        assert!((steps as f32 * STEP + fs.accumulator - fed).abs() < 1e-6);
        assert!(steps >= 3, "6 × ⅔ step owes 4 steps (±f32 eps), got {steps}");
    }

    #[test]
    fn burst_catchup_never_exceeds_cap() {
        let mut fs = FixedStep::new(STEP);
        // A 500 ms stall owes 30 steps; the cap is 5.
        assert_eq!(fs.tick(0.5), 5);
        // The stall's leftover time is dropped, so the next frame is not
        // another full burst — that is the whole point of the guard.
        let next = fs.tick(0.0);
        assert!(next <= 1, "after a capped burst the loop recovers, got {next}");
        assert_eq!(fs.tick(STEP), 1, "steady state resumes");
    }

    #[test]
    fn alpha_always_in_unit_halfopen() {
        let mut fs = FixedStep::new(STEP);
        for dt in [0.0, STEP / 7.0, STEP * 0.999, STEP, STEP * 1.5, 2.0] {
            fs.tick(dt);
            let a = fs.alpha();
            assert!((0.0..1.0).contains(&a), "alpha {a} for dt {dt}");
        }
    }
}
