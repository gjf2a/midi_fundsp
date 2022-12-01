pub mod io;
pub mod sounds;

use fundsp::hacker::{
    adsr_live, clamp01, envelope2, midi_hz, moog_q, shared, var, xerp, An, AudioUnit64, FrameMul,
    Net64, Shared, Var,
};
use std::fmt::Debug;

pub const MAX_MIDI_VALUE: u8 = 127;

pub type SynthFunc = dyn Fn(&SharedMidiState) -> Box<dyn AudioUnit64> + Send + Sync;

#[derive(Clone)]
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
            control: Default::default(),
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
    pub fn bent_pitch(&self) -> Net64 {
        Net64::wrap(Box::new(var(&self.pitch_bend) * var(&self.pitch)))
    }

    pub fn control_var(&self) -> An<Var<f64>> {
        var(&self.control)
    }

    pub fn volume(&self, adjuster: Box<dyn AudioUnit64>) -> Net64 {
        Net64::bin_op(
            Net64::wrap(Box::new(var(&self.velocity))),
            Net64::wrap(adjuster),
            FrameMul::new(),
        )
    }

    pub fn assemble_sound(
        &self,
        synth: Box<dyn AudioUnit64>,
        adjuster: Box<dyn AudioUnit64>,
    ) -> Box<dyn AudioUnit64> {
        Box::new(Net64::bin_op(
            Net64::pipe_op(self.bent_pitch(), Net64::wrap(synth)),
            self.volume(adjuster),
            FrameMul::new(),
        ))
    }

    pub fn on(&mut self, pitch: u8, velocity: u8) {
        self.pitch.set_value(midi_hz(pitch as f64));
        self.velocity
            .set_value(velocity as f64 / MAX_MIDI_VALUE as f64);
        self.control.set_value(1.0);
    }

    pub fn off(&mut self) {
        self.control.set_value(-1.0);
    }

    pub fn bend(&mut self, bend: u16) {
        self.pitch_bend.set_value(pitch_bend_factor(bend));
    }
}

/// Algorithm is from here: https://sites.uci.edu/camp2014/2014/04/30/managing-midi-pitchbend-messages/
/// Converts MIDI pitch-bend message to +/- 1 semitone.
fn pitch_bend_factor(bend: u16) -> f64 {
    2.0_f64.powf(((bend as f64 - 8192.0) / 8192.0) / 12.0)
}

#[derive(Copy, Clone, Debug)]
pub struct Adsr {
    pub attack: f64,
    pub decay: f64,
    pub sustain: f64,
    pub release: f64,
}

impl Adsr {
    pub fn boxed(&self, state: &SharedMidiState) -> Box<dyn AudioUnit64> {
        let control = state.control_var();
        Box::new(control >> adsr_live(self.attack, self.decay, self.sustain, self.release))
    }

    pub fn net64ed(&self, state: &SharedMidiState) -> Net64 {
        Net64::wrap(self.boxed(state))
    }

    pub fn timed_sound(&self, timed_sound: Box<dyn AudioUnit64>, state: &SharedMidiState) -> Net64 {
        Net64::pipe_op(
            Net64::stack_op(state.bent_pitch(), self.net64ed(state)),
            Net64::wrap(timed_sound),
        )
    }
}

// It works, but I'm trying to avoid macros.
#[allow(unused)]
macro_rules! op {
    ($fn:expr) => {
        envelope2(move |_, n| $fn(n))
    };
}

pub fn simple_sound(state: &SharedMidiState, synth: Box<dyn AudioUnit64>) -> Box<dyn AudioUnit64> {
    let control = state.control_var();
    state.assemble_sound(
        synth,
        Box::new(control >> envelope2(move |_, n| clamp01(n))),
    )
}

pub fn adsr_timed_moog(
    state: &SharedMidiState,
    source: Box<dyn AudioUnit64>,
    adsr: Adsr,
) -> Box<dyn AudioUnit64> {
    Box::new(Net64::pipe_op(
        Net64::stack_op(
            Net64::wrap(source),
            Net64::pipe_op(
                adsr.net64ed(state),
                Net64::wrap(Box::new(envelope2(move |_, n| xerp(1100.0, 11000.0, n)))),
            ),
        ),
        Net64::wrap(Box::new(moog_q(0.6))),
    ))
}
