---
layout: default
title: Frequently Asked Questions
---

# Frequently asked questions

## Is GamerRUST ready for a production game?

Not yet. Version `0.1.0` is intended for experimentation, learning, and prototypes. The API can change without a deprecation period while core design decisions are still being validated.

## Which platforms are supported?

The project is continuously built and tested on Linux, macOS, and Windows. Actual GPU, audio, and gamepad behavior also depends on the backend libraries, operating system, hardware, and drivers.

## Is GamerRUST on crates.io?

No. Depend on the Git repository for now and pin a tested commit revision. See [Getting started](getting-started.md#use-gamerrust-from-another-project).

## Why does Linux fail while compiling ALSA or `libudev`?

The audio and gamepad dependencies require native development headers. On Ubuntu or Debian:

```sh
sudo apt-get install libasound2-dev libudev-dev
```

Use your distribution's equivalent ALSA and `libudev` development packages elsewhere.

## Why does the first build take so long?

Graphics, windowing, audio, text, and gamepad support bring a substantial dependency graph. Cargo compiles it on the first build and reuses the artifacts for later builds. A release build also performs more optimization.

## Can I build a 2D-only game?

Yes. Use `SpriteBatch`, `Texture`, and `Camera2D`; you do not need to create or use the 3D renderer.

## Can I build a 3D game?

The current renderer supports meshes, perspective cameras, model transforms, directional lighting, depth testing, generated cube data, and OBJ loading. It is intentionally small and does not yet aim to match a full production 3D engine.

## Does text require bundled font assets?

No. The current text system uses system fonts, so the examples do not need a font file in the repository. Text appearance can vary between systems.

## Does audio require asset files?

No. `Sound` can be generated from samples or waveforms, and the audio example is entirely procedural. Buffered sound data can also be used by applications.

## How do I report a rendering bug?

Open an issue with your operating system, Rust version, GPU model, graphics driver version, the failing example, full logs, and a minimal reproduction if possible. Remove personal or sensitive information from diagnostic output.

## Where should new documentation go?

Use `/docs` for guides and conceptual material, the root `README.md` for the project overview, and rustdoc comments for API-specific behavior. Keeping guides in `/docs` allows the same content to back a GitHub Pages site and the repository Wiki.
