---
layout: default
title: GamerRUST Documentation
---

# GamerRUST Documentation

GamerRUST is a batteries-included 2D and 3D game engine for Rust. It provides one cohesive API for windowing, rendering, input, audio, timing, text, mesh loading, and 2D collision.

> GamerRUST is currently at version `0.1.0`. It is suitable for experimentation, learning, and prototypes, but its API may change.

## Start here

- [Getting started](getting-started.md) — install prerequisites, clone the project, and run your first demo.
- [Examples](examples.md) — understand every bundled example and its controls.
- [Architecture](architecture.md) — learn how engine modules and third-party libraries fit together.
- [FAQ](faq.md) — troubleshoot common setup and usage questions.
- [Contributing](https://github.com/FeverDream-dev/GamerRUST/blob/master/CONTRIBUTING.md) — prepare a change and run the project checks.

## At a glance

| Need | GamerRUST module | Main types |
| --- | --- | --- |
| Engine lifecycle | `app` | `Engine`, `Game`, `Context` |
| Frame timing | `time` | `Time`, `FixedStep` |
| 2D camera | `camera` | `Camera2D` |
| 3D camera | `camera` | `Camera3D` |
| Textures and graphics context | `gfx` | `Graphics`, `Texture` |
| Sprite rendering | `gfx::batch2d` | `SpriteBatch`, `Sprite` |
| 3D rendering | `gfx::render3d` | `Renderer3d`, `Draw3d`, `DirectionalLight` |
| Text rendering | `gfx::text` | `TextContext`, `Text` |
| Input | `input` | `Input`, `Key`, `GamepadButton`, `GamepadAxis` |
| Audio | `audio` | `AudioSystem`, `Sound`, `MusicHandle` |
| Mesh data | `mesh` | `MeshData`, `Mesh` |
| 2D collision | `collision2d` | `Aabb`, `Circle`, `swept_aabb` |

## Project links

- [Source repository](https://github.com/FeverDream-dev/GamerRUST)
- [Issue tracker](https://github.com/FeverDream-dev/GamerRUST/issues)
- [CI workflow](https://github.com/FeverDream-dev/GamerRUST/actions/workflows/ci.yml)
- [MIT license](https://github.com/FeverDream-dev/GamerRUST/blob/master/LICENSE-MIT)
- [Apache-2.0 license](https://github.com/FeverDream-dev/GamerRUST/blob/master/LICENSE-APACHE)
