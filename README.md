# GamerRUST

<div align="center">

**A batteries-included 2D and 3D game engine for Rust.**

[![CI](https://github.com/FeverDream-dev/GamerRUST/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/FeverDream-dev/GamerRUST/actions/workflows/ci.yml)
[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](https://www.rust-lang.org/tools/install)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

[Getting started](docs/getting-started.md) · [Examples](docs/examples.md) · [Architecture](docs/architecture.md) · [FAQ](docs/faq.md) · [Contributing](CONTRIBUTING.md)

</div>

GamerRUST gives game developers one cohesive API over proven Rust building blocks, including `winit`, `wgpu`, `rodio`, `gilrs`, `glam`, `glyphon`, and `tobj`. It owns the engine API and integration layer so you can focus on game logic instead of wiring subsystems together.

> [!IMPORTANT]
> GamerRUST is an early-stage `0.1.0` project. It is usable for experimentation, learning, and prototypes, but APIs may change as the engine matures.

## Why GamerRUST?

- **One approachable facade:** windowing, rendering, input, audio, timing, and collisions share a consistent API.
- **2D and 3D in one crate:** batch sprites for 2D games or render lit, depth-tested meshes in 3D.
- **Useful defaults:** a fixed 60 Hz update loop, interpolated drawing, system-font text, and unified input work out of the box.
- **Self-contained examples:** every demo generates what it needs at runtime, so no external asset pack is required.
- **Tested across platforms:** CI formats, lints, tests, and builds examples on Linux, macOS, and Windows.

## Features

| System | Main API | What it provides |
| --- | --- | --- |
| Application | `Engine`, `Game`, `Context` | Window creation, fixed-step updates, interpolated drawing |
| 2D rendering | `SpriteBatch`, `Sprite`, `Texture`, `Camera2D` | Instanced sprite batching and camera transforms |
| 3D rendering | `Renderer3d`, `MeshData`, `Camera3D`, `DirectionalLight` | Mesh uploads, OBJ loading, lighting, and depth testing |
| Text | `TextContext`, `Text` | System-font text with color, size, and positioning |
| Input | `Input`, `Key`, `GamepadButton` | Keyboard, mouse, and gamepad state through one interface |
| Audio | `AudioSystem`, `Sound` | Generated or buffered sound effects and a music channel |
| Collision | `Aabb`, `Circle`, `swept_aabb` | Overlap tests, minimum translation vectors, and swept collision |
| Timing | `Time`, `FixedStep` | Frame timing, FPS tracking, and deterministic update steps |

## Quick start

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) 1.85 or newer
- Linux only: ALSA and `libudev` development headers

```sh
# Ubuntu / Debian
sudo apt-get install libasound2-dev libudev-dev
```

### Run an example

```sh
git clone https://github.com/FeverDream-dev/GamerRUST.git
cd GamerRUST
cargo run --example window
```

Press <kbd>Esc</kbd> to close the window. See the [examples guide](docs/examples.md) for every demo and its controls.

## Minimal game

```rust
use gamerrust::prelude::*;

struct MyGame {
    batch: Option<SpriteBatch>,
    player: Vec2,
}

impl Game for MyGame {
    fn init(ctx: &mut Context) -> Self {
        let batch = ctx.gfx().map(|gfx| {
            let pixels = vec![255; 16 * 16 * 4];
            let texture = Texture::from_rgba(gfx, 16, 16, &pixels);
            SpriteBatch::new(gfx, texture)
        });

        Self {
            batch,
            player: Vec2::ZERO,
        }
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) {
        let Some(input) = ctx.input() else { return };

        if input.pressed(Key::Escape) {
            ctx.exit();
            return;
        }
        if input.pressed(Key::D) {
            self.player.x += 200.0 * dt;
        }
    }

    fn draw(&mut self, ctx: &mut Context, _alpha: f32) {
        let Some(gfx) = ctx.gfx() else { return };
        gfx.clear_color([0.05, 0.05, 0.10, 1.0]);

        if let Some(batch) = self.batch.as_mut() {
            let (width, height) = gfx.size();
            batch.set_camera(&Camera2D::default(), width as f32, height as f32);
            batch.push(Sprite::new(self.player, Vec2::splat(64.0)));
            batch.flush(gfx);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Engine::new()
        .title("My GamerRUST game")
        .size(800, 600)
        .run::<MyGame>()
}
```

## Examples

```sh
cargo run --example window
cargo run --example bunnymark_2d
cargo run --example input_demo
cargo run --example audio_demo
cargo run --example text_demo
cargo run --example cube_3d
```

The [examples guide](docs/examples.md) explains the purpose and controls of each demo.

## Documentation

- [Documentation home](docs/index.md)
- [Getting started](docs/getting-started.md)
- [Examples and controls](docs/examples.md)
- [Architecture](docs/architecture.md)
- [Frequently asked questions](docs/faq.md)
- [Contributing](CONTRIBUTING.md)

The `/docs` directory is ready to be published as a GitHub Pages site and can also be used as the source for the repository Wiki.

## Development

Run the same checks used by CI:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --examples
```

Headless GPU tests render sprites, text, and 3D scenes offscreen and assert the resulting pixels. For contribution workflow and platform notes, read [CONTRIBUTING.md](CONTRIBUTING.md).

## Project status

GamerRUST currently targets desktop development on Linux, macOS, and Windows. The project is young, the public API is not yet stable, and there is no crates.io release yet. Pin a Git revision when depending on the repository from another project.

## License

GamerRUST is dual-licensed under either of the following, at your option:

- [Apache License 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

Unless you explicitly state otherwise, contributions intentionally submitted for inclusion in this project are licensed under the same terms.
