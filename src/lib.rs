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

pub mod app;
pub mod audio;
pub mod camera;
pub mod collision2d;
pub mod gfx;
pub mod input;
pub mod mesh;
pub mod time;

/// Everything a game needs in one `use gamerrust::prelude::*;`.
///
/// Later milestones append their headline types here (`Camera2D`, `Input`,
/// `mesh`, ...) as they land.
pub mod prelude {
    pub use crate::app::{Context, Engine, Game};
    pub use crate::audio::AudioSystem;
    pub use crate::camera::{Camera2D, Camera3D};
    pub use crate::collision2d::{swept_aabb, Aabb, Circle};
    pub use crate::gfx::batch2d::{Sprite, SpriteBatch};
    pub use crate::gfx::render3d::{DirectionalLight, Draw3d, Renderer3d};
    pub use crate::gfx::text::{Text, TextContext};
    pub use crate::gfx::Graphics;
    pub use crate::gfx::Texture;
    pub use crate::input::{
        ButtonState, GamepadAxis, GamepadButton, Input, Key, MouseButton as EngineMouseButton,
    };
    pub use crate::time::{FixedStep, Time};
    pub use glam::{Vec2, Vec3};
}
