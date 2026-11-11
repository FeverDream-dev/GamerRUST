//! The app skeleton: [`Game`] trait, [`Context`], and the [`Engine`] builder
//! that drives a winit window on a fixed 60 Hz timestep.
//!
//! The loop is the classic accumulator pattern: real frame time is fed to
//! [`FixedStep`], which yields 0..N fixed `update` ticks per frame; `draw`
//! then runs once with the interpolation `alpha`. Events not consumed by the
//! loop itself reach the game via [`Game::event`].

use std::error::Error;
use std::time::Instant;

use crate::time::{FixedStep, Time};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

/// Fixed simulation rate in Hz. One constant feeds both `Time` and `FixedStep`.
const FIXED_HZ: f32 = 60.0;

/// Handle to the engine's subsystems, handed to every [`Game`] call.
///
/// A fresh `Context` is built per engine phase (event, each fixed update,
/// draw); it borrows the live subsystems from the runner, so it is cheap and
/// short-lived. Fields grow as subsystems land (`gfx` in M2, `input` in M4).
pub struct Context<'a> {
    /// Snapshot of the frame timing (delta, elapsed, fps, fixed step).
    pub time: Time,
    pub(crate) exit: &'a mut bool,
    window: Option<&'a Window>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(time: Time, exit: &'a mut bool, window: Option<&'a Window>) -> Self {
        Self { time, exit, window }
    }

    /// Ask the engine to shut down after the current phase.
    pub fn exit(&mut self) {
        *self.exit = true;
    }

    /// The active window. `None` only in headless scenarios.
    pub fn window(&self) -> Option<&Window> {
        self.window
    }
}

/// A game driven by [`Engine`]: `init` once, then `update` at a fixed 60 Hz
/// step and `draw` once per frame with the interpolation `alpha` in `[0,1)`.
pub trait Game: Sized {
    /// Build the initial state. Called once, after the window exists.
    fn init(ctx: &mut Context) -> Self;
    /// Fixed-step simulation update; `dt` is always `ctx.time.fixed_delta`.
    fn update(&mut self, ctx: &mut Context, dt: f32);
    /// Per-frame render call; blend simulation state by `alpha`.
    fn draw(&mut self, ctx: &mut Context, alpha: f32);
    /// Raw winit window event (default: ignored) — escape hatch for things
    /// the unified input doesn't model (IME, drag-drop, resize semantics).
    fn event(&mut self, _ctx: &mut Context, _event: &WindowEvent) {}
}

/// Builder for the engine:
/// `Engine::new().title("My game").size(1280, 720).run::<MyGame>()`.
pub struct Engine {
    title: String,
    size: (u32, u32),
    /// Consumed by the graphics context (M2) to pick the present mode.
    #[allow(dead_code)] // read by `Graphics` starting M2
    vsync: bool,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            title: "GamerRUST".into(),
            size: (1280, 720),
            vsync: true,
        }
    }
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Window inner size in logical pixels.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    /// Vsync on = `Fifo` present mode (default), off = `AutoNoVsync` (M2).
    pub fn vsync(mut self, on: bool) -> Self {
        self.vsync = on;
        self
    }

    /// Run the event loop with game `G` until the game exits or the window
    /// closes. Honors `GAMERRUST_EXIT_AFTER=<frames>` (smoke tests/CI):
    /// exits cleanly after that many drawn frames.
    pub fn run<G: Game>(self) -> Result<(), Box<dyn Error>> {
        let exit_after = std::env::var("GAMERRUST_EXIT_AFTER")
            .ok()
            .and_then(|v| v.parse::<u64>().ok());
        let event_loop = EventLoop::new()?;

        let mut runner: Runner<G> = Runner {
            cfg: self,
            game: None,
            window: None,
            fixed: FixedStep::new(1.0 / FIXED_HZ),
            time: Time::with_fixed_hz(FIXED_HZ),
            last: Instant::now(),
            frames: 0,
            exit_after,
            exit: false,
        };
        event_loop.run_app(&mut runner)?;
        Ok(())
    }
}

/// Everything the event loop owns: window, game state, timing.
struct Runner<G: Game> {
    cfg: Engine,
    game: Option<G>,
    window: Option<Window>,
    fixed: FixedStep,
    time: Time,
    last: Instant,
    /// Frames drawn so far (for `GAMERRUST_EXIT_AFTER`).
    frames: u64,
    exit_after: Option<u64>,
    exit: bool,
}

impl<G: Game> Runner<G> {
    /// One fixed update tick. Disjoint-field borrows let `game` and the
    /// context coexist without cloning state.
    fn update_step(&mut self) {
        // `let` pattern to keep the mutable game borrow scoped.
        if let Some(game) = self.game.as_mut() {
            let mut ctx = Context::new(self.time, &mut self.exit, self.window.as_ref());
            game.update(&mut ctx, self.time.fixed_delta);
        }
    }

    fn draw_frame(&mut self, event_loop: &ActiveEventLoop) {
        self.frames += 1;
        if let Some(game) = self.game.as_mut() {
            let alpha = self.fixed.alpha();
            let mut ctx = Context::new(self.time, &mut self.exit, self.window.as_ref());
            game.draw(&mut ctx, alpha);
        }
        if self.exit {
            event_loop.exit();
        }
    }
}

impl<G: Game> ApplicationHandler for Runner<G> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attrs = winit::window::Window::default_attributes()
                .with_title(self.cfg.title.clone())
                .with_inner_size(winit::dpi::LogicalSize::new(
                    self.cfg.size.0,
                    self.cfg.size.1,
                ));
            self.window = Some(event_loop.create_window(attrs).expect("create window"));
        }
        if self.game.is_none() {
            let mut ctx = Context::new(self.time, &mut self.exit, self.window.as_ref());
            self.game = Some(G::init(&mut ctx));
        }
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.exit = true,
            WindowEvent::RedrawRequested => self.draw_frame(_event_loop),
            ref other => {
                if let Some(game) = self.game.as_mut() {
                    let mut ctx = Context::new(self.time, &mut self.exit, self.window.as_ref());
                    game.event(&mut ctx, other);
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.exit || self.exit_after == Some(self.frames) {
            event_loop.exit();
            return;
        }
        let now = Instant::now();
        // Clamp: a debugger pause must not dump 30 s into the accumulator.
        let dt = ((now - self.last).as_secs_f32()).min(0.25);
        self.last = now;

        self.time.delta = dt;
        self.time.elapsed += dt;
        if dt > 1e-6 {
            self.time.fps = self.time.fps * 0.9 + (1.0 / dt) * 0.1;
        }

        let steps = self.fixed.tick(dt);
        for _ in 0..steps {
            self.update_step();
        }

        if self.exit {
            event_loop.exit();
            return;
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}
