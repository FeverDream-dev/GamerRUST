//! M5 audio demo — self-contained (engine `app` module not landed yet, so
//! this drives winit directly; core can swap it onto `Engine::run` later).
//!
//! Keys: `1` sine blip, `2` sawtooth buzz, `3` noise-ish chord, `M` music
//! (looped arpeggio) toggle pause, `N` stop music, `ESC` quit.
//! `GAMERRUST_EXIT_AFTER=<secs>` auto-quits (used by smoke checks/CI).

use std::time::Instant;

use gamerrust::audio::{MusicHandle, Sound};
use gamerrust::prelude::AudioSystem;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// A short looping arpeggio built from pure generators — proves `from_samples`.
fn arpeggio() -> Sound {
    let mut samples = Vec::new();
    for &freq in &[440.0, 554.37, 659.25, 880.0, 659.25, 554.37] {
        samples.extend(Sound::sine(freq, 0.18).samples);
        samples.extend(Sound::silence(0.02).samples);
    }
    Sound::from_samples(samples, gamerrust::audio::DEFAULT_SAMPLE_RATE)
}

struct Demo {
    window: Option<Window>,
    audio: AudioSystem,
    music: Option<MusicHandle>,
    last_tick: Instant,
    exit_after: Option<Instant>,
    started: Instant,
}

impl Demo {
    fn key(&mut self, key: &str, event_loop: &ActiveEventLoop) {
        match key {
            "1" => self.audio.play_sound(&Sound::sine(880.0, 0.15), 0.4),
            "2" => self.audio.play_sound(&Sound::sawtooth(110.0, 0.4), 0.25),
            "3" => {
                // Two detuned saws: quick hand-rolled "chord".
                let a = Sound::sawtooth(220.0, 0.3);
                let b = Sound::sawtooth(226.0, 0.3);
                let mixed: Vec<f32> = a
                    .samples
                    .iter()
                    .zip(&b.samples)
                    .map(|(x, y)| (x + y) * 0.5)
                    .collect();
                self.audio
                    .play_sound(&Sound::from_samples(mixed, a.sample_rate), 0.3);
            }
            "m" => match &self.music {
                Some(h) if h.state().status() == gamerrust::audio::Status::Paused => {
                    h.resume();
                    println!("music resumed");
                }
                Some(h) if h.is_active() => {
                    h.pause();
                    println!("music paused");
                }
                _ => {
                    self.music = Some(self.audio.play_music(arpeggio(), true));
                    println!("music started (looped)");
                }
            },
            "n" => {
                if let Some(h) = &self.music {
                    h.stop();
                    println!("music stopped");
                }
            }
            _ => {}
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
        let _ = event_loop;
    }
}

impl ApplicationHandler for Demo {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.window = Some(
                event_loop
                    .create_window(
                        winit::window::Window::default_attributes()
                            .with_title("GamerRUST — audio demo (1/2/3 sound, M music, ESC quit)"),
                    )
                    .expect("create window"),
            );
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        state: ElementState::Pressed,
                        repeat: false,
                        ..
                    },
                ..
            } => match logical_key {
                Key::Named(NamedKey::Escape) => event_loop.exit(),
                Key::Character(ref c) => self.key(c.to_lowercase().as_str(), event_loop),
                _ => {}
            },
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = now - self.last_tick;
        self.last_tick = now;
        if let Some(h) = &self.music {
            h.advance(dt.as_secs_f32());
        }
        if let Some(deadline) = self.exit_after {
            if now >= deadline {
                println!(
                    "GAMERRUST_EXIT_AFTER reached after {:.1}s — bye",
                    now.duration_since(self.started).as_secs_f32()
                );
                _event_loop.exit();
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    println!("audio demo: keys 1/2/3 = sounds, M = music start/pause/resume, N = stop, ESC = quit");

    let started = Instant::now();
    let exit_after = std::env::var("GAMERRUST_EXIT_AFTER")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .map(|secs| started + std::time::Duration::from_secs_f32(secs));

    let event_loop = EventLoop::new()?;
    // Wake exactly at the exit deadline (if any) instead of waiting for OS events.
    event_loop.set_control_flow(match exit_after {
        Some(deadline) => ControlFlow::WaitUntil(deadline),
        None => ControlFlow::Wait,
    });

    let mut demo = Demo {
        window: None,
        audio: AudioSystem::new(),
        music: None,
        last_tick: Instant::now(),
        exit_after,
        started,
    };

    // No asset files, no device required to start: sounds generate at runtime
    // and the device opens lazily on first keypress.
    demo.audio.play_sound(&Sound::sine(440.0, 0.1), 0.3);

    event_loop.run_app(&mut demo)?;
    Ok(())
}
