//! M1 demo — a window with a sine-animated clear color, on the fixed-step
//! engine loop. Real wgpu clears land in M2; until then the animation is the
//! title text (updated at draw rate) proving the loop runs and interpolates.
//!
//! Keys: `ESC` quit. `GAMERRUST_EXIT_AFTER=<frames>` exits cleanly after N
//! drawn frames (smoke checks/CI).

use gamerrust::prelude::*;

struct WindowDemo {
    frames: u64,
}

impl Game for WindowDemo {
    fn init(_ctx: &mut Context) -> Self {
        Self { frames: 0 }
    }

    fn update(&mut self, _ctx: &mut Context, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, _alpha: f32) {
        self.frames += 1;
        // Sine pulse of the window title until wgpu clears exist (M2).
        let t = ctx.time.elapsed;
        let v = (t * 2.0 * std::f32::consts::PI * 0.5).sin() * 0.5 + 0.5;
        if let Some(w) = ctx.window() {
            w.set_title(&format!(
                "GamerRUST — window demo | frame {} | anim {v:.3} | {:.0} fps",
                self.frames, ctx.time.fps
            ));
        }
        if self.frames % 15 == 0 {
            log::info!(
                "frame {} elapsed {:.2}s anim {v:.3} fps {:.1}",
                self.frames,
                ctx.time.elapsed,
                ctx.time.fps
            );
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    Engine::new()
        .title("GamerRUST — window demo (ESC quit)")
        .size(800, 600)
        .run::<WindowDemo>()
}
