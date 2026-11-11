//! M7 demo — text rendering: FPS counter, moving banner, colored text.
//! ESC exits.

use gamerrust::gfx::text::{Text, TextContext};
use gamerrust::prelude::*;

struct TextDemo {
    text: Option<TextContext>,
    t: f32,
}

impl Game for TextDemo {
    fn init(ctx: &mut Context) -> Self {
        let text = ctx.gfx().map(|gfx| TextContext::new(gfx));
        Self { text, t: 0.0 }
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) {
        let Some(input) = ctx.input() else { return };
        if input.pressed(Key::Escape) {
            ctx.exit();
        }
        self.t += dt;
    }

    fn draw(&mut self, ctx: &mut Context, _alpha: f32) {
        let fps = ctx.time.fps;
        let Some(gfx) = ctx.gfx() else { return };
        let Some(text) = self.text.as_mut() else {
            return;
        };
        gfx.clear_color([0.05, 0.05, 0.1, 1.0]);

        text.draw(Text::new(
            Vec2::new(16.0, 16.0),
            format!("GamerRUST text — {:.0} fps", fps),
        ));
        text.draw(
            Text::new(
                Vec2::new(16.0, 56.0),
                "The quick brown fox jumps over the lazy dog 0123456789",
            )
            .size(20.0),
        );
        // Moving banner: sine-wave x offset.
        let x = 200.0 + (self.t * 2.0).sin() * 150.0;
        text.draw(
            Text::new(Vec2::new(x, 300.0), "GAMERRUST")
                .size(64.0)
                .color([255, 180, 40, 255]),
        );
        text.draw(
            Text::new(Vec2::new(16.0, 400.0), "ESC to quit")
                .size(16.0)
                .color([160, 160, 160, 255]),
        );
        text.flush(gfx);
    }
}

fn main() {
    env_logger::init();
    Engine::new()
        .title("GamerRUST text demo")
        .size(800, 600)
        .run::<TextDemo>()
        .expect("engine run");
}
