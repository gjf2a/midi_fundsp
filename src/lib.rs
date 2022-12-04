//! This crate enables the construction of synthesizers with live MIDI input and sound synthesis
//! using [fundsp](https://crates.io/crates/fundsp).
//!
//! It is organized as follows:
//! * The crate root contains functions and data structures useful for constucting [fundsp](https://crates.io/crates/fundsp)
//!   sounds.
//!   * [MIDI input messages](https://www.midi.org/specifications-old/item/table-1-summary-of-midi-message) are
//!   converted into `SharedMidiState` objects that translate the sounds represented by those messages into
//!   [fundsp `Shared` atomic variables](https://docs.rs/fundsp/0.10.0/fundsp/audionode/struct.Shared.html).
//!   * `SynthFunc` functions translate `SharedMidiState` objects into specific [fundsp](https://crates.io/crates/fundsp) audio graphs.
//! * The `io` module contains functions and data types for obtaining messages from MIDI devices and playing  
//!   [fundsp](https://crates.io/crates/fundsp) audio graphs through the computer's speakers.
//! * The `sound_builders` module contains functions that wrap [fundsp](https://crates.io/crates/fundsp) audio graphs
//!   into `SynthFunc` functions with a variety of properties.
//! * The `sounds` module contains `SynthFunc` functions that produce a variety of live sounds.
//!
//! The following [example programs](https://github.com/gjf2a/midi_fundsp/tree/master/examples) show how these components
//! interact to produce a working synthesizer:
//! * [`basic_demo.rs`](https://github.com/gjf2a/midi_fundsp/blob/master/examples/basic_demo.rs) opens the first MIDI
//! device it finds and plays a simple triangle waveform sound in response to MIDI events.
//! * [`stereo_demo.rs`](https://github.com/gjf2a/midi_fundsp/blob/master/examples/stereo_demo.rs) also opens the first MIDI
//! device it finds. It plays notes below middle C through the left speaker using a Moog Pulse sound, and notes
//! at Middle C or higher through the right speaker using a Moog Triangle sound.
//! * [`choice_demo.rs`](https://github.com/gjf2a/midi_fundsp/blob/master/examples/choice_demo.rs) allows the user to choose
//! one from among all connected MIDI devices. The user can then choose any sound from the `sounds` module for the program's
//! response to MIDI events.

pub mod io;
pub mod sound_builders;
pub mod sounds;

use fundsp::hacker::{midi_hz, shared, var, An, AudioUnit64, FrameMul, Net64, Shared, Var};
use std::fmt::Debug;
use std::sync::Arc;
use std::time::{Instant, Duration};

/// MIDI values for pitch and velocity range from 0 to 127.
pub const MAX_MIDI_VALUE: u8 = 127;

/// Control value in response to `Note On` event.
pub const CONTROL_ON: f64 = 1.0;

/// Control value in response to `Note Off` event.
pub const CONTROL_OFF: f64 = -1.0;

/// `SynthFunc` objects translate `SharedMidiState` values into [fundsp](https://crates.io/crates/fundsp) audio graphs.
pub type SynthFunc = Arc<dyn Fn(&SharedMidiState) -> Box<dyn AudioUnit64> + Send + Sync>;

#[derive(Clone)]
/// `SharedMidiState` objects represent as [fundsp `Shared` atomic variables](https://docs.rs/fundsp/0.10.0/fundsp/audionode/struct.Shared.html)
/// the following MIDI events:
/// * `Note On`
/// * `Note Off`
/// * `Pitch Bend`
pub struct SharedMidiState {
    pitch: Shared<f64>,
    velocity: Shared<f64>,
    control: Shared<f64>,
    pitch_bend: Shared<f64>,
}

impl Default for SharedMidiState {
    fn default() -> Self {
        Self {
            pitch: Default::default(),
            velocity: Default::default(),
            control: shared(CONTROL_OFF),
            pitch_bend: shared(1.0),
        }
    }
}

impl Debug for SharedMidiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedMidiState")
            .field("pitch", &self.pitch.value())
            .field("velocity", &self.velocity.value())
            .field("control", &self.control.value())
            .field("pitch_bend", &self.pitch_bend.value())
            .finish()
    }
}

impl SharedMidiState {
    /// Returns the most recent `Note On` pitch, modified by the most recent `Pitch Bend` event.
    pub fn bent_pitch(&self) -> Net64 {
        Net64::wrap(Box::new(var(&self.pitch_bend) * var(&self.pitch)))
    }

    /// Returns `CONTROL_ON` if `Note On` is the most recent event for this pitch, and `CONTROL_OFF` otherwise.
    pub fn control_var(&self) -> An<Var<f64>> {
        var(&self.control)
    }

    /// Returns the current volume.
    ///
    /// The volume is determined from the velocity of the most recent `Note On` event in combination with the
    /// output from the `adjuster`. The `adjuster` should use `control_var()` to determine whether the most recent
    /// event is `Note On` or `Note Off`, and adjust the volume accordingly, whether it is a sudden cutoff or
    /// a gradual release.
    pub fn volume(&self, adjuster: Box<dyn AudioUnit64>) -> Net64 {
        Net64::bin_op(
            Net64::wrap(Box::new(var(&self.velocity))),
            Net64::wrap(adjuster),
            FrameMul::new(),
        )
    }

    /// Pipes the current `bent_pitch()` into `synth`, then multiplies by `volume(adjuster)` to
    /// produce the final sound.
    pub fn assemble_unpitched_sound(
        &self,
        synth: Box<dyn AudioUnit64>,
        adjuster: Box<dyn AudioUnit64>,
    ) -> Box<dyn AudioUnit64> {
        self.assemble_pitched_sound(
            Box::new(Net64::pipe_op(self.bent_pitch(), Net64::wrap(synth))),
            adjuster,
        )
    }

    pub fn assemble_pitched_sound(
        &self,
        pitched_sound: Box<dyn AudioUnit64>,
        adjuster: Box<dyn AudioUnit64>,
    ) -> Box<dyn AudioUnit64> {
        Box::new(Net64::bin_op(
            Net64::wrap(pitched_sound), 
            self.volume(adjuster),
            FrameMul::new(),
        ))
    }

    /// Encodes a MIDI `Note On` event.
    pub fn on(&self, pitch: u8, velocity: u8) {
        self.pitch.set_value(midi_hz(pitch as f64));
        self.velocity
            .set_value(velocity as f64 / MAX_MIDI_VALUE as f64);
        self.control.set_value(CONTROL_ON);
    }

    /// Encodes a MIDI `Note Off` event.
    pub fn off(&self) {
        self.control.set_value(CONTROL_OFF);
    }

    /// Encodes a MIDI `Pitch Bend` event.
    ///
    /// Algorithm from: https://sites.uci.edu/camp2014/2014/04/30/managing-midi-pitchbend-messages/
    /// Converts MIDI pitch-bend message to +/- 1 semitone.
    pub fn bend(&self, bend: u16) {
        self.pitch_bend.set_value(pitch_bend_factor(bend));
    }
}

/// Algorithm from: https://sites.uci.edu/camp2014/2014/04/30/managing-midi-pitchbend-messages/
/// Converts MIDI pitch-bend message to +/- 1 semitone.
pub fn pitch_bend_factor(bend: u16) -> f64 {
    2.0_f64.powf(((bend as f64 - 8192.0) / 8192.0) / 12.0)
}

#[derive(Debug)]
pub struct SoundTestResult {
    total: f64,
    count: usize,
    min: f64,
    max: f64
}

impl SoundTestResult {
    pub fn add_value(&mut self, value: f64) {
        self.count += 1;
        self.total += value;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
    }

    pub fn report(&self) {
        println!("{} ({}..{})", self.total / self.count as f64, self.min, self.max);
    }
}

impl Default for SoundTestResult {
    fn default() -> Self {
        Self { total: Default::default(), count: Default::default(), min: f64::MAX, max: f64::MIN }
    }
}

impl SoundTestResult {
    pub fn test(sound: Arc<dyn Fn(&SharedMidiState) -> Box<dyn AudioUnit64> + Send + Sync>) -> Self {
        const SAMPLE_RATE: f64 = 44100.0;
        const DURATION: f64 = 5.0;
        const SLEEP_TIME: f64 = 1.0 / SAMPLE_RATE;

        let mut result = Self::default();
        let state = SharedMidiState::default();
        let mut sound = sound(&state);
        sound.reset(Some(SAMPLE_RATE));
        let mut next_value = move || sound.get_mono();
        let start = Instant::now();
        state.on(60, 127);
        while start.elapsed().as_secs_f64() < DURATION {
            result.add_value(next_value());
            std::thread::sleep(Duration::from_secs_f64(SLEEP_TIME));
        }
        result
    }
}