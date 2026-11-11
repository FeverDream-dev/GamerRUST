---
layout: default
title: Getting Started
---

# Getting started

This guide gets the repository building and opens a GamerRUST example window.

## Requirements

- Rust 1.85 or newer
- Git
- A desktop environment with a GPU supported by `wgpu`

Check your Rust installation:

```sh
rustc --version
cargo --version
```

Install or update Rust with [rustup](https://rustup.rs/) if the compiler is older than 1.85.

### Linux native packages

Audio uses ALSA and gamepad support uses `libudev`, so their development headers must be available before Cargo builds the dependency tree.

```sh
# Ubuntu / Debian
sudo apt-get update
sudo apt-get install libasound2-dev libudev-dev
```

Package names vary across Linux distributions. Install the development packages that provide ALSA and `libudev` headers on your distribution.

## Clone and run

```sh
git clone https://github.com/FeverDream-dev/GamerRUST.git
cd GamerRUST
cargo run --example window
```

The first build downloads and compiles the Rust dependencies, so it may take a few minutes. A window with an animated background should appear. Press <kbd>Esc</kbd> to exit.

## Try the engine systems

```sh
cargo run --example bunnymark_2d  # 2D sprite batching
cargo run --example input_demo    # keyboard, mouse, and gamepad
cargo run --example audio_demo    # sound effects and music
cargo run --example text_demo     # system-font text
cargo run --example cube_3d       # lit 3D rendering
```

See [Examples](examples.md) for the controls.

## Use GamerRUST from another project

GamerRUST is not yet published on crates.io. During early development, depend on the Git repository and pin a revision so your build remains reproducible:

```toml
[dependencies]
gamerrust = { git = "https://github.com/FeverDream-dev/GamerRUST.git", rev = "<commit-sha>" }
```

Replace `<commit-sha>` with the commit you have tested. Update intentionally after reviewing API changes.

## Verify a checkout

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --examples
```

These are the same checks run by CI on Linux, macOS, and Windows.

## Next steps

- Read [Architecture](architecture.md) to understand the engine layers.
- Explore [Examples](examples.md) and adapt the nearest demo.
- Check the [FAQ](faq.md) if setup or a native dependency fails.
- Read [Contributing](https://github.com/FeverDream-dev/GamerRUST/blob/master/CONTRIBUTING.md) before proposing a change.
