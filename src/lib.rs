//! # GamerRUST
//!
//! Batteries-included 2D+3D game engine for Rust, by FeverDream.
//!
//! One facade crate over proven building blocks (`winit`, `wgpu`, `rodio`,
//! `gilrs`, `glam`, `glyphon`, `tobj`) — we own the API and glue, not the
//! GPU plumbing.
//!
//! ## Modules
//!
//! - [`collision2d`] — AABB + circle overlap, resolution, and swept tests
//!
//! <!-- Later milestones plug in here:
//! - `app` — Game trait, engine builder, fixed-timestep loop (M1)
//! - `gfx` — wgpu context, 2D batcher, 3D pipeline, text (M2/M3/M7/M8)
//! - `input` — unified keyboard/mouse/gamepad state (M4)
//! - `audio` — sound/music over rodio (M5)
//! - `mesh` — .obj loading (M8)
//! -->

pub mod collision2d;

/// Everything a game needs in one `use gamerrust::prelude::*;`.
///
/// Later milestones append their headline types here (`Engine`, `Game`,
/// `Camera2D`, `Input`, `Audio`, ...) as they land.
pub mod prelude {
    pub use crate::collision2d::{swept_aabb, Aabb, Circle};
    pub use glam::Vec2;
}
