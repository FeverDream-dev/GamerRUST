# GamerRUST

Batteries-included 2D+3D game engine for Rust, by FeverDream.

<!-- Badges — uncomment once CI is live on the FeverDream-dev org repo (M9):
[![CI](https://github.com/FeverDream-dev/GamerRUST/actions/workflows/ci.yml/badge.svg)](https://github.com/FeverDream-dev/GamerRUST/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/gamerrust.svg)](https://crates.io/crates/gamerrust)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT_OR_Apache--2.0-blue.svg)](#license)
-->

## Quickstart

```rust
use gamerrust::prelude::*;

fn main() {
    // Engine::new().run::<MyGame>() arrives with milestone M1.
    // Collision is available today:
    let a = Aabb { min: Vec2::new(0.0, 0.0), max: Vec2::new(10.0, 10.0) };
    let b = Aabb { min: Vec2::new(5.0, 5.0), max: Vec2::new(15.0, 15.0) };
    assert!(a.overlap_aabb(&b));
}
```

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
