---
layout: default
title: Architecture
---

# Architecture

GamerRUST is a facade crate: it integrates focused Rust libraries behind one game-oriented API instead of reimplementing operating-system, GPU, audio, and device backends.

## Runtime flow

1. `Engine::run` creates the event loop, window, graphics context, input state, audio state, and timing state.
2. `Game::init` receives a mutable `Context` and creates game-owned resources.
3. `Game::update` advances simulation at a fixed 60 Hz step.
4. `Game::draw` renders the current frame and receives an interpolation alpha.
5. Window and device events update engine state and may be forwarded through `Game::event`.

This separation keeps simulation updates deterministic while drawing can follow the display refresh rate.

## Module map

| Module | Responsibility | Key dependencies |
| --- | --- | --- |
| `app` | Engine builder, lifecycle, window/event integration, shared `Context` | `winit`, `pollster` |
| `time` | Fixed-step scheduling, frame delta, elapsed time, FPS | `instant` |
| `gfx` | GPU instance, surface, device, queue, textures, frame encoding | `wgpu`, `image` |
| `gfx::batch2d` | Instanced sprite batching and 2D camera uniforms | `wgpu`, `glam`, `bytemuck` |
| `gfx::render3d` | Mesh pipeline, model transforms, camera and light uniforms, depth buffer | `wgpu`, `glam`, `bytemuck` |
| `gfx::text` | Queued text layout and rendering | `glyphon` |
| `camera` | 2D view/projection and 3D perspective camera math | `glam` |
| `input` | Keyboard, mouse, and gamepad state with edge transitions | `winit`, `gilrs` |
| `audio` | Sound data, generated waveforms, playback, and music control | `rodio` |
| `mesh` | CPU mesh data, cube generation, OBJ loading, GPU upload | `tobj`, `wgpu` |
| `collision2d` | AABB and circle overlap, resolution, swept AABB | `glam` |

## Ownership boundaries

The engine owns:

- the public game-oriented API;
- lifecycle and subsystem integration;
- resource wrappers and sensible defaults;
- tests and examples that verify the integrated behavior.

The underlying libraries own:

- native window and event backends (`winit`);
- graphics APIs and GPU abstraction (`wgpu`);
- audio device and decoding infrastructure (`rodio`);
- gamepad backends (`gilrs`);
- vector and matrix math (`glam`);
- text shaping and rasterization (`glyphon`);
- OBJ parsing (`tobj`).

Keeping this boundary explicit makes it easier to upgrade backends without exposing every backend detail as part of GamerRUST's public API.

## Testing strategy

The project combines:

- unit tests for pure logic such as timing and collision;
- headless GPU tests that render offscreen and inspect pixels;
- compilation of all examples on every CI platform;
- formatting and warning-free Clippy checks.

Interactive examples remain important manual smoke tests for windowing, devices, and real GPU presentation.

## Public API

Most games can begin with:

```rust
use gamerrust::prelude::*;
```

The prelude exports the headline types used in the main loop. More specialized types remain available through their modules. Because the project is at `0.1.0`, consumers should expect public API refinement and pin a tested Git revision.
