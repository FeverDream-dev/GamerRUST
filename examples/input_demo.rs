//! M4 demo — unified input: square position follows mouse / WASD / stick.
//!
//! Rendering today is clear-color only (sprites are M3), so the moving
//! "square" is shown by tinting the clear color by position, plus the live
//! position in the window title. ESC exits.

use gamerrust::prelude::*;

struct InputDemo {
    pos: Vec2,
}

impl Game for InputDemo {
    fn init(_ctx: &mut Context) -> Self {
        Self { pos: Vec2::ZERO }
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) {
        let Some(input) = ctx.input() else { return };

        if input.pressed(Key::Escape) {
            ctx.exit();
            return;
        }

        let mut vel = Vec2::ZERO;
        if input.pressed(Key::W) {
            vel.y -= 1.0;
        }
        if input.pressed(Key::S) {
            vel.y += 1.0;
        }
        if input.pressed(Key::A) {
            vel.x -= 1.0;
        }
        if input.pressed(Key::D) {
            vel.x += 1.0;
        }
        // Gamepad left stick adds to keyboard velocity.
        for id in input.gamepad_ids() {
            vel.x += input.gamepad_axis(id, GamepadAxis::LeftStickX);
            vel.y += input.gamepad_axis(id, GamepadAxis::LeftStickY);
        }
        if vel.length() > 1.0 {
            vel = vel.normalize();
        }
        self.pos += vel * 300.0 * dt;

        // Mouse: holding left button drags the square to the cursor.
        if input.mouse_button(EngineMouseButton::Left).held {
            self.pos = input.mouse_pos();
        }
    }

    fn draw(&mut self, ctx: &mut Context, _alpha: f32) {
        if let Some(w) = ctx.window() {
            w.set_title(&format!(
                "GamerRUST input — pos {:.0},{:.0}",
                self.pos.x, self.pos.y
            ));
        }
        if let Some(gfx) = ctx.gfx() {
            // Tint clear by normalized position: visible feedback, no sprites.
            let t = (self.pos / 600.0).clamp(Vec2::ZERO, Vec2::ONE);
            gfx.clear_color([t.x, 0.2, t.y, 1.0]);
        }
    }
}

fn main() {
    env_logger::init();
    Engine::new()
        .title("GamerRUST input demo")
        .size(800, 600)
        .run::<InputDemo>()
        .expect("engine run");
}
