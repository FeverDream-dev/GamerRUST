//! Audio: sound effects and music over [`rodio`].
//!
//! Zero-friction design: sounds are in-memory `f32` sample buffers ([`Sound`])
//! produced by pure generators (sine, sawtooth, silence) — no asset files
//! needed, and the whole playback state machine ([`MusicState`]) is testable
//! without an audio device.

use std::sync::{Arc, Mutex};

/// One channel of `f32` samples in `[-1, 1]`.
pub type Samples = Vec<f32>;

/// Default sample rate the generators use when a caller doesn't care.
pub const DEFAULT_SAMPLE_RATE: u32 = 44_100;

/// An in-memory mono sound: samples at a known sample rate.
///
/// rodio resamples on playback if the device rate differs, so generators can
/// produce at any convenient rate.
#[derive(Debug, Clone, PartialEq)]
pub struct Sound {
    /// Mono samples in `[-1, 1]`.
    pub samples: Samples,
    /// Samples per second the buffer was generated at.
    pub sample_rate: u32,
}

impl Sound {
    /// Wrap an existing sample buffer.
    pub fn from_samples(samples: Samples, sample_rate: u32) -> Self {
        Self {
            samples,
            sample_rate,
        }
    }

    /// A sine wave of `freq_hz` lasting `duration_s`.
    pub fn sine(freq_hz: f32, duration_s: f32) -> Self {
        Self::sine_at(freq_hz, duration_s, DEFAULT_SAMPLE_RATE)
    }

    /// [`Sound::sine`] with an explicit sample rate.
    pub fn sine_at(freq_hz: f32, duration_s: f32, sample_rate: u32) -> Self {
        Self::from_samples(
            generate(duration_s, sample_rate, |t| {
                (2.0 * std::f32::consts::PI * freq_hz * t).sin()
            }),
            sample_rate,
        )
    }

    /// A sawtooth wave of `freq_hz` lasting `duration_s` (buzzier than sine;
    /// good for engines and lasers).
    pub fn sawtooth(freq_hz: f32, duration_s: f32) -> Self {
        Self::sawtooth_at(freq_hz, duration_s, DEFAULT_SAMPLE_RATE)
    }

    /// [`Sound::sawtooth`] with an explicit sample rate.
    pub fn sawtooth_at(freq_hz: f32, duration_s: f32, sample_rate: u32) -> Self {
        Self::from_samples(
            generate(duration_s, sample_rate, |t| {
                let phase = (freq_hz * t).fract();
                2.0 * phase - 1.0
            }),
            sample_rate,
        )
    }

    /// Exactly `duration_s` of silence — gaps, padding, tests.
    pub fn silence(duration_s: f32) -> Self {
        Self::silence_at(duration_s, DEFAULT_SAMPLE_RATE)
    }

    /// [`Sound::silence`] with an explicit sample rate.
    pub fn silence_at(duration_s: f32, sample_rate: u32) -> Self {
        let n = sample_count(duration_s, sample_rate);
        Self::from_samples(vec![0.0; n], sample_rate)
    }

    /// Duration in seconds.
    pub fn duration_secs(&self) -> f32 {
        self.samples.len() as f32 / self.sample_rate as f32
    }

    /// Total sample count (the loop/end bound for [`MusicState::advance`]).
    pub fn total_samples(&self) -> u64 {
        self.samples.len() as u64
    }

    fn buffer(&self) -> rodio::buffer::SamplesBuffer {
        rodio::buffer::SamplesBuffer::new(1, self.sample_rate, self.samples.clone())
    }
}

/// Sample count for `duration_s` at `sample_rate` (rounded, at least 0).
fn sample_count(duration_s: f32, sample_rate: u32) -> usize {
    (duration_s.max(0.0) * sample_rate as f32).round() as usize
}

/// Sample `f(t)` (t in seconds) for `duration_s` at `sample_rate`,
/// clamping amplitude to `[-1, 1]`.
fn generate(duration_s: f32, sample_rate: u32, f: impl Fn(f32) -> f32) -> Samples {
    (0..sample_count(duration_s, sample_rate))
        .map(|i| f(i as f32 / sample_rate as f32).clamp(-1.0, 1.0))
        .collect()
}

/// Clamp a raw volume into `[0, 1]`.
pub fn clamp_volume(raw: f32) -> f32 {
    raw.clamp(0.0, 1.0)
}

/// Pure playback state for a music channel — no device involved.
///
/// Position is modeled in samples and advanced by [`MusicState::advance`] so
/// the whole machine (play → pause → resume → stop, looping, volume) is unit
/// testable headlessly. [`AudioSystem`] mirrors these transitions onto the
/// real rodio sink when a device exists; the sample position is advisory
/// (the sink loops itself).
#[derive(Debug, Clone, PartialEq)]
pub struct MusicState {
    status: Status,
    volume: f32,
    looped: bool,
    position_samples: u64,
    total_samples: u64,
}

/// Lifecycle of [`MusicState`]. "Stopped" and "idle" are the same resting
/// state — there is nothing to distinguish once the queue is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Idle,
    Playing,
    Paused,
}

impl MusicState {
    /// Fresh idle state at full volume for a track of `total_samples`.
    pub fn new(total_samples: u64) -> Self {
        Self {
            status: Status::Idle,
            volume: 1.0,
            looped: false,
            position_samples: 0,
            total_samples,
        }
    }

    /// Start playing from the beginning. Volume set while idle is kept.
    pub fn play(&mut self, looped: bool) {
        self.status = Status::Playing;
        self.looped = looped;
        self.position_samples = 0;
    }

    /// Pause if playing; pausing while idle/paused is a no-op.
    pub fn pause(&mut self) {
        if self.status == Status::Playing {
            self.status = Status::Paused;
        }
    }

    /// Resume if paused; resuming while idle/playing is a no-op.
    pub fn resume(&mut self) {
        if self.status == Status::Paused {
            self.status = Status::Playing;
        }
    }

    /// Stop and forget position; stopping while idle is a no-op.
    pub fn stop(&mut self) {
        self.status = Status::Idle;
        self.looped = false;
        self.position_samples = 0;
    }

    /// Set volume, clamped to `[0, 1]`.
    pub fn set_volume(&mut self, raw: f32) {
        self.volume = clamp_volume(raw);
    }

    /// Current clamped volume.
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// Advance the playhead by `dt_samples`.
    ///
    /// Only moves while [`Status::Playing`]. Looping wraps modulo the track
    /// length; non-looping plays out and falls idle.
    pub fn advance(&mut self, dt_samples: u64) {
        if self.status != Status::Playing {
            return;
        }
        self.position_samples = self.position_samples.saturating_add(dt_samples);
        if self.position_samples >= self.total_samples {
            if self.looped && self.total_samples > 0 {
                self.position_samples %= self.total_samples;
            } else {
                self.stop();
            }
        }
    }

    /// Current status.
    pub fn status(&self) -> Status {
        self.status
    }

    /// Whether a track is loaded and not stopped (Playing or Paused).
    pub fn is_active(&self) -> bool {
        self.status != Status::Idle
    }

    /// Whether set to loop.
    pub fn looped(&self) -> bool {
        self.looped
    }

    /// Playhead in samples (0 when idle).
    pub fn position_samples(&self) -> u64 {
        self.position_samples
    }
}

/// The single music channel: shared pure state plus the live rodio sink
/// (absent on headless machines). Shared by [`AudioSystem`] and every
/// [`MusicHandle`], so dropping handles never stops the music.
// ponytail: single music sink — add channels when a game needs crossfade.
struct MusicChannel {
    state: Mutex<MusicState>,
    sink: Mutex<Option<rodio::Sink>>,
    sample_rate: Mutex<u32>,
}

impl std::fmt::Debug for MusicChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The sink isn't Debug; the pure state is what matters.
        f.debug_struct("MusicChannel")
            .field("state", &self.state.lock().unwrap())
            .field("sample_rate", &self.sample_rate.lock().unwrap())
            .finish_non_exhaustive()
    }
}

impl MusicChannel {
    fn new() -> Self {
        Self {
            state: Mutex::new(MusicState::new(0)),
            sink: Mutex::new(None),
            sample_rate: Mutex::new(DEFAULT_SAMPLE_RATE),
        }
    }

    /// Run `f` on the live sink if one exists.
    fn with_sink<R>(&self, f: impl FnOnce(&rodio::Sink) -> R) -> Option<R> {
        self.sink.lock().unwrap().as_ref().map(f)
    }
}

/// Audio output: fire-and-forget sound effects plus one controllable music
/// channel, over rodio.
///
/// Constructing is free; the output device is opened lazily on first play,
/// so games (and CI) run silently on machines with no audio device.
pub struct AudioSystem {
    // ponytail: lazy device open — headless CI/tests never touch cpal. If a
    // game needs explicit device selection or reconnection after unplug,
    // grow an init() here.
    stream: Mutex<Option<rodio::OutputStream>>,
    music: Arc<MusicChannel>,
}

impl Default for AudioSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioSystem {
    /// Create without opening any device.
    pub fn new() -> Self {
        Self {
            stream: Mutex::new(None),
            music: Arc::new(MusicChannel::new()),
        }
    }

    /// Open the default output stream on first use, then run `f` on its
    /// mixer. Returns `None` (silently) when no device is available.
    fn with_mixer<R>(&self, f: impl FnOnce(&rodio::mixer::Mixer) -> R) -> Option<R> {
        let mut stream = self.stream.lock().unwrap();
        if stream.is_none() {
            *stream = rodio::OutputStreamBuilder::open_default_stream().ok();
        }
        stream.as_ref().map(|s| f(s.mixer()))
    }

    /// Play a one-shot sound effect at `volume` (clamped to `[0, 1]`).
    /// Multiple one-shots overlap freely. No-op without an audio device.
    pub fn play_sound(&self, sound: &Sound, volume: f32) {
        self.with_mixer(|mixer| {
            let sink = rodio::Sink::connect_new(mixer);
            sink.set_volume(clamp_volume(volume));
            sink.append(sound.buffer());
            // Fire-and-forget: detach so the sink's drop doesn't stop playback.
            sink.detach();
        });
    }

    /// Start `sound` on the music channel, replacing whatever played before.
    /// Returns a [`MusicHandle`] for volume/pause/resume/stop control.
    /// Without an audio device the state machine still runs (silently).
    pub fn play_music(&mut self, sound: Sound, looped: bool) -> MusicHandle {
        *self.music.sample_rate.lock().unwrap() = sound.sample_rate;
        {
            let mut state = self.music.state.lock().unwrap();
            let volume = state.volume();
            *state = MusicState::new(sound.total_samples());
            state.set_volume(volume);
            state.play(looped);
        }
        let buffer = sound.buffer();
        let sink = self.with_mixer(|mixer| {
            let sink = rodio::Sink::connect_new(mixer);
            let state = self.music.state.lock().unwrap();
            sink.set_volume(state.volume());
            if state.looped() {
                use rodio::Source as _;
                sink.append(buffer.repeat_infinite());
            } else {
                sink.append(buffer);
            }
            sink
        });
        *self.music.sink.lock().unwrap() = sink;
        self.music_handle()
    }

    /// A fresh handle to the music channel (e.g. after losing the old one).
    pub fn music_handle(&self) -> MusicHandle {
        MusicHandle {
            music: Arc::clone(&self.music),
        }
    }
}

/// Remote control for the music channel started by [`AudioSystem::play_music`].
///
/// Cloning is cheap; every clone steers the same single channel.
#[derive(Debug, Clone)]
pub struct MusicHandle {
    music: Arc<MusicChannel>,
}

impl MusicHandle {
    /// Set music volume, clamped to `[0, 1]`.
    pub fn set_volume(&self, volume: f32) {
        let clamped = clamp_volume(volume);
        self.music.state.lock().unwrap().set_volume(clamped);
        self.music.with_sink(|s| s.set_volume(clamped));
    }

    /// Current clamped volume.
    pub fn volume(&self) -> f32 {
        self.music.state.lock().unwrap().volume()
    }

    /// Pause the music (no-op unless playing).
    pub fn pause(&self) {
        self.music.state.lock().unwrap().pause();
        self.music.with_sink(|s| s.pause());
    }

    /// Resume the music (no-op unless paused).
    pub fn resume(&self) {
        self.music.state.lock().unwrap().resume();
        self.music.with_sink(|s| s.play());
    }

    /// Stop the music and forget the position (no-op when idle).
    pub fn stop(&self) {
        self.music.state.lock().unwrap().stop();
        // ponytail: rodio Sink::stop is terminal — the next play_music builds
        // a fresh sink anyway, so just clearing the queue here is enough.
        self.music.with_sink(|s| s.stop());
    }

    /// Whether the channel is playing or paused.
    pub fn is_active(&self) -> bool {
        self.music.state.lock().unwrap().is_active()
    }

    /// Advance the pure playhead model by `dt_seconds`. The real sink loops
    /// on its own; this keeps [`MusicState`] honest for UI/seek logic.
    pub fn advance(&self, dt_seconds: f32) {
        let rate = *self.music.sample_rate.lock().unwrap();
        let dt_samples = (dt_seconds.max(0.0) * rate as f32) as u64;
        self.music.state.lock().unwrap().advance(dt_samples);
    }

    /// Snapshot of the pure state (for tests and debug UIs).
    pub fn state(&self) -> MusicState {
        self.music.state.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_has_exact_length_and_bounds() {
        let s = Sound::sine_at(440.0, 0.5, 44_100);
        assert_eq!(s.samples.len(), 22_050);
        assert!(s.samples.iter().all(|&x| (-1.0..=1.0).contains(&x)));
        assert_eq!(s.duration_secs(), 0.5);
    }

    #[test]
    fn sine_matches_reference_values() {
        // 100 Hz at 1 kHz: sample i is sin(pi * i / 5).
        let s = Sound::sine_at(100.0, 1.0, 1_000);
        assert!((s.samples[0] - 0.0).abs() < 1e-6);
        assert!((s.samples[1] - (std::f32::consts::PI / 5.0).sin()).abs() < 1e-6);
        // 10 samples/cycle never land exactly on the crest: peak sample is
        // sin(2π/5) ≈ 0.951 (f32 phase rounding drifts ~2e-5).
        let peak = s.samples.iter().cloned().fold(0.0_f32, f32::max);
        assert!((peak - (2.0 * std::f32::consts::PI / 5.0).sin()).abs() < 1e-4);
    }

    #[test]
    fn sawtooth_ramps_minus_one_to_one() {
        // 1 Hz at 4 Hz: phases 0, .25, .5, .75 -> -1, -0.5, 0, +0.5.
        let s = Sound::sawtooth_at(1.0, 1.0, 4);
        let want = [-1.0, -0.5, 0.0, 0.5];
        for (got, want) in s.samples.iter().zip(want) {
            assert!((got - want).abs() < 1e-6, "got {got}, want {want}");
        }
    }

    #[test]
    fn silence_is_exact_zeros() {
        let s = Sound::silence_at(0.25, 8_000);
        assert_eq!(s.samples.len(), 2_000);
        assert!(s.samples.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn generators_clamp_amplitude() {
        // A sine at Nyquist-ish freq can't exceed 1 by construction, but the
        // clamp must hold for any generator output.
        let s = Sound::sine_at(1000.0, 1.0, 2);
        assert!(s.samples.iter().all(|&x| (-1.0..=1.0).contains(&x)));
    }

    #[test]
    fn volume_clamps_to_unit_range() {
        let mut m = MusicState::new(100);
        m.set_volume(1.7);
        assert_eq!(m.volume(), 1.0);
        m.set_volume(-0.3);
        assert_eq!(m.volume(), 0.0);
        m.set_volume(0.42);
        assert_eq!(m.volume(), 0.42);
        assert_eq!(clamp_volume(9.0), 1.0);
        assert_eq!(clamp_volume(-1.0), 0.0);
    }

    #[test]
    fn play_pause_resume_stop_cycle() {
        let mut m = MusicState::new(1_000);
        assert_eq!(m.status(), Status::Idle);
        m.play(true);
        assert_eq!(m.status(), Status::Playing);
        assert!(m.is_active());
        m.advance(100);
        m.pause();
        assert_eq!(m.status(), Status::Paused);
        assert_eq!(m.position_samples(), 100, "pause keeps position");
        m.advance(500);
        assert_eq!(m.position_samples(), 100, "paused does not advance");
        m.resume();
        assert_eq!(m.status(), Status::Playing);
        m.advance(50);
        assert_eq!(m.position_samples(), 150);
        m.stop();
        assert_eq!(m.status(), Status::Idle);
        assert_eq!(m.position_samples(), 0);
        assert!(!m.is_active());
    }

    #[test]
    fn pause_resume_stop_are_no_ops_when_idle_or_wrong_state() {
        let mut m = MusicState::new(100);
        m.pause(); // idle pause
        assert_eq!(m.status(), Status::Idle);
        m.resume(); // idle resume
        assert_eq!(m.status(), Status::Idle);
        m.play(false);
        m.resume(); // resume while playing
        assert_eq!(m.status(), Status::Playing);
        m.pause();
        m.pause(); // double pause
        assert_eq!(m.status(), Status::Paused);
        m.stop();
        m.stop(); // double stop
        assert_eq!(m.status(), Status::Idle);
    }

    #[test]
    fn loop_wraps_position_to_start() {
        let mut m = MusicState::new(100);
        m.play(true);
        m.advance(90);
        m.advance(30); // wraps: 120 % 100 = 20
        assert_eq!(m.status(), Status::Playing);
        assert_eq!(m.position_samples(), 20);
        assert!(m.looped());
    }

    #[test]
    fn non_loop_plays_out_and_goes_idle() {
        let mut m = MusicState::new(100);
        m.play(false);
        m.advance(99);
        assert_eq!(m.status(), Status::Playing);
        m.advance(1); // exactly to the end
        assert_eq!(m.status(), Status::Idle);
        assert_eq!(m.position_samples(), 0);
        // Overshoot behaves the same.
        m.play(false);
        m.advance(500);
        assert_eq!(m.status(), Status::Idle);
    }

    #[test]
    fn advance_partial_and_idle_no_op() {
        let mut m = MusicState::new(1_000);
        m.advance(123); // idle: nothing happens
        assert_eq!(m.status(), Status::Idle);
        m.play(false);
        m.advance(10);
        m.advance(15);
        assert_eq!(m.position_samples(), 25);
    }

    #[test]
    fn play_keeps_volume_set_while_idle() {
        let mut m = MusicState::new(50);
        m.set_volume(0.3);
        m.play(true);
        assert_eq!(m.volume(), 0.3);
    }

    #[test]
    fn audio_system_constructs_without_device() {
        // Must not open any device or panic (headless CI).
        let _sys = AudioSystem::new();
        let _sys2 = AudioSystem::default();
    }
}
