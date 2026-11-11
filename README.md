# GamerRUST

Batteries-included 2D + 3D game engine for Rust, by [FeverDream](https://github.com/FeverDream-dev).

One facade crate over proven building blocks (`winit`, `wgpu`, `rodio`, `gilrs`, `glam`, `glyphon`, `tobj`) — we own the API and glue, not the GPU plumbing. Windows open, sprites batch, text shapes, audio plays, gamepads connect — out of the box.

## Quickstart

```rust
use gamerrust::prelude::*;

struct MyGame {
    batch: Option<SpriteBatch>,
    bunny: Vec2,
}

impl Game for MyGame {
    fn init(ctx: &mut Context) -> Self {
        let batch = ctx.gfx().map(|gfx| {
            let tex = Texture::from_rgba(gfx, 16, 16, &my_pixel_art());
            SpriteBatch::new(gfx, tex)
        });
        Self { batch, bunny: Vec2::ZERO }
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) {
        let Some(input) = ctx.input() else { return };
        if input.pressed(Key::Escape) { ctx.exit(); return; }
        if input.pressed(Key::D) { self.bunny.x += 200.0 * dt; }
    }

    fn draw(&mut self, ctx: &mut Context, _alpha: f32) {
        let Some(gfx) = ctx.gfx() else { return };
        gfx.clear_color([0.05, 0.05, 0.1, 1.0]);
        if let Some(batch) = self.batch.as_mut() {
            batch.set_camera(&Camera2D::default(), 800.0, 600.0);
            batch.push(Sprite::new(self.bunny, Vec2::new(64.0, 64.0)));
            batch.flush(gfx);
        }
    }
}

fn main() {
    Engine::new().title("My game").size(800, 600).run::<MyGame>().unwrap();
}
```

## What's inside

| System | API | Notes |
|---|---|---|
| App loop | `Engine`, `Game`, `Context` | fixed 60 Hz update, interpolated draw |
| 2D | `SpriteBatch`, `Sprite`, `Texture`, `Camera2D` | instanced batching, pixel-verified placement |
| 3D | `Renderer3d`, `MeshData`, `Camera3D`, `DirectionalLight` | .obj loading, lit + depth-tested pipeline |
| Text | `TextContext`, `Text` | system fonts, zero assets |
| Input | `Input`, `Key`, `GamepadButton` | keyboard/mouse/gamepad, just-pressed edges |
| Audio | `AudioSystem`, `Sound` | procedural + buffer sounds, music channel |
| Collision | `Aabb`, `Circle`, `swept_aabb` | 2D overlap, MTV, swept tunneling |

## Examples

```sh
cargo run --example window        # app loop + animated clear
cargo run --example bunnymark_2d  # sprite batcher stress test (hold SPACE)
cargo run --example input_demo    # keyboard/mouse/gamepad
cargo run --example audio_demo    # procedural sounds
cargo run --example text_demo     # text rendering
cargo run --example cube_3d       # lit 3D cube
```

## Testing

Every subsystem ships with runnable checks — unit tests plus headless GPU
pixel tests (sprites/text/3D render offscreen and the pixels are asserted):

```sh
cargo test
```

## License

Dual-licensed MIT OR Apache-2.0.
