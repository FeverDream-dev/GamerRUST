//! M1/M2 demo — a window on the fixed-step engine loop with a sine-animated
//! wgpu clear color. ESC quits.
//!
//! `GAMERRUST_EXIT_AFTER=<frames>` exits cleanly after N drawn frames
//! (smoke checks/CI).

use gamerrust::prelude::*;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{Key, NamedKey};

struct WindowDemo {
    frames: u64,
    color: [f32; 4],
}

impl Game for WindowDemo {
    fn init(_ctx: &mut Context) -> Self {
        Self {
            frames: 0,
            color: [0.1, 0.2, 0.3, 1.0],
        }
    }

    fn update(&mut self, _ctx: &mut Context, _dt: f32) {
        // Simulation advances the hue phase; draw interpolates nothing yet,
        // but the split is exercised from day one.
    }

    fn draw(&mut self, ctx: &mut Context, _alpha: f32) {
        self.frames += 1;
        // Three offset sines: an RGB color cycling once every ~4 s.
        let t = ctx.time.elapsed;
        let s = |phase: f32| (t * std::f32::consts::FRAC_PI_2 + phase).sin() * 0.5 + 0.5;
        self.color = [s(0.0), s(2.1), s(4.2), 1.0];
        if let Some(gfx) = ctx.gfx() {
            gfx.clear_color(self.color);
        }
        if self.frames % 15 == 0 {
            log::info!(
                "frame {} elapsed {:.2}s color {:?} fps {:.0}",
                self.frames,
                ctx.time.elapsed,
                self.color,
                ctx.time.fps
            );
        }
    }

    fn event(&mut self, ctx: &mut Context, event: &WindowEvent) {
        if let WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    logical_key: Key::Named(NamedKey::Escape),
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } = event
        {
            ctx.exit();
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
