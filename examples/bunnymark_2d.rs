//! M3 demo — bunnymark: sprite batcher stress test.
//!
//! Sprites bounce around; hold SPACE to add more (up to 100k), the title
//! shows count + fps. ESC exits. Uses a generated checker texture so the
//! demo needs no asset files.

use gamerrust::prelude::*;

const MAX_BUNNIES: usize = 100_000;

struct Bunnymark {
    batch: Option<SpriteBatch>,
    positions: Vec<Vec2>,
    velocities: Vec<Vec2>,
}

impl Game for Bunnymark {
    fn init(ctx: &mut Context) -> Self {
        let batch = ctx.gfx().map(|gfx| {
            let tex = Texture::from_rgba(gfx, 16, 16, &checker_texture(16, 16));
            SpriteBatch::new(gfx, tex)
        });
        let mut s = Self {
            batch,
            positions: Vec::new(),
            velocities: Vec::new(),
        };
        s.spawn(1000);
        s
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) {
        let Some(input) = ctx.input() else { return };
        if input.pressed(Key::Escape) {
            ctx.exit();
            return;
        }
        if input.pressed(Key::Space) && self.positions.len() < MAX_BUNNIES {
            self.spawn(1000);
        }
        let (w, h) = ctx
            .window()
            .map(|win| win.inner_size())
            .map(|s| (s.width as f32, s.height as f32))
            .unwrap_or((800.0, 600.0));
        for (p, v) in self.positions.iter_mut().zip(&mut self.velocities) {
            *p += *v * dt;
            if p.x < 0.0 || p.x > w {
                v.x = -v.x;
                p.x = p.x.clamp(0.0, w);
            }
            if p.y < 0.0 || p.y > h {
                v.y = -v.y;
                p.y = p.y.clamp(0.0, h);
            }
        }
    }

    fn draw(&mut self, ctx: &mut Context, _alpha: f32) {
        if let Some(w) = ctx.window() {
            w.set_title(&format!(
                "GamerRUST bunnymark — {} sprites, {:.0} fps",
                self.positions.len(),
                ctx.time.fps
            ));
        }
        let Some(batch) = self.batch.as_mut() else {
            return;
        };
        let Some(gfx) = ctx.gfx() else { return };
        gfx.clear_color([0.08, 0.08, 0.12, 1.0]);
        let cam = Camera2D::default();
        let (w, h) = gfx.size();
        batch.set_camera(&cam, w as f32, h as f32);
        for &p in &self.positions {
            batch.push(Sprite::new(p, Vec2::new(16.0, 16.0)));
        }
        batch.flush(gfx);
    }
}

impl Bunnymark {
    fn spawn(&mut self, n: usize) {
        // Deterministic LCG; no RNG dep needed.
        // ponytail: LCG seeded per-spawn-batch; swap for rand when a game
        // needs real randomness.
        let mut seed = (self.positions.len() as u32)
            .wrapping_mul(2654435761)
            .wrapping_add(1);
        for _ in 0..n {
            if self.positions.len() >= MAX_BUNNIES {
                break;
            }
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let x = (seed >> 8) as f32 % 800.0;
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let y = (seed >> 8) as f32 % 600.0;
            self.positions.push(Vec2::new(x, y));
            self.velocities.push(Vec2::new(
                if seed & 1 == 0 { 120.0 } else { -120.0 },
                160.0 + (seed >> 16) as f32 % 80.0,
            ));
        }
    }
}

/// 16x16 two-tone checker, RGBA8.
fn checker_texture(w: u32, h: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let light = (x / 4 + y / 4) % 2 == 0;
            let c = if light {
                [240, 200, 80, 255]
            } else {
                [180, 60, 60, 255]
            };
            data.extend_from_slice(&c);
        }
    }
    data
}

fn main() {
    env_logger::init();
    Engine::new()
        .title("GamerRUST bunnymark")
        .size(800, 600)
        .run::<Bunnymark>()
        .expect("engine run");
}
