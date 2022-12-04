use std::sync::Arc;

use fundsp::hacker::{dsf_saw, pulse, sine, triangle, AudioUnit64, dsf_square};

use crate::sound_builders::{simple_sound, Adsr, ProgramTable};
use crate::SharedMidiState;

macro_rules! program_table {
    ($( ($s:expr, $f:expr)),* ) => {vec![$(($s.to_owned(), Arc::new($f)),)*]}
}

pub fn options() -> ProgramTable {
    program_table![
        ("Simple Triangle", simple_triangle),
        ("Triangle", adsr_triangle),
        ("Pulse", adsr_pulse),
        ("DSF Saw", adsr_dsf_saw),
        ("DSF Square", adsr_dsf_square),
        ("Moog Triangle", moog_triangle),
        ("Moog Pulse", moog_pulse),
        ("Sine", adsr_sine)
    ]
}

pub fn moogs() -> ProgramTable {
    program_table![("Moog Triangle", moog_triangle), ("Moog Pulse", moog_pulse)]
}

pub fn simple_triangle(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    simple_sound(state, Box::new(triangle()))
}

pub const ADSR1: Adsr = Adsr {
    attack: 0.1,
    decay: 0.2,
    sustain: 0.4,
    release: 0.4,
};

pub const ADSR2: Adsr = Adsr {
    attack: 0.1,
    decay: 0.4,
    sustain: 0.4,
    release: 0.6,
};

/*
// The pluck() function is weird and I will need some help with it.
//
pub fn adsr_pluck(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    let pitch = state.bent_pitch();
    let volume = state.volume(ADSR1.boxed(state));
    let excitation = shared(1.0);

    Box::new(Net64::pipe_op(
        Net64::stack_op(
            Net64::wrap(Box::new(var(&excitation))),
            pitch),
        Net64::wrap(Box::new(envelope3(|_,excitation, frequency| pluck(frequency, 0.5, 0.5))))))
}
*/

/*
// Another noble attempt.

pub fn adsr_pluck(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    let pitch = state.pitch_shared();
    state.assemble_sound(Box::new(var_fn(pitch, |p| zero() >> pluck(p, 0.8, 0.8))), adjuster)
}
*/

pub fn adsr_triangle(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    state.assemble_sound(Box::new(triangle()), ADSR1.boxed(state))
}

pub fn adsr_sine(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    state.assemble_sound(Box::new(sine()), ADSR1.boxed(state))
}

pub fn adsr_pulse(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    ADSR2.assemble_timed(Box::new(pulse()), state)
}

pub fn adsr_dsf_saw(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    ADSR2.assemble_timed(Box::new(dsf_saw()), state)
}

pub fn adsr_dsf_square(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    ADSR2.assemble_timed(Box::new(dsf_square()), state)
}

pub fn moog_triangle(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    state.assemble_sound(
        Box::new(ADSR2.timed_moog(Box::new(triangle()), state)),
        ADSR2.boxed(state),
    )
}

pub fn moog_pulse(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    state.assemble_pitched_sound(
        Box::new(ADSR2.timed_moog(Box::new(ADSR2.timed_sound(Box::new(pulse()), state)), state)),
        ADSR2.boxed(state),
    )
}

pub fn moog_dsf_square(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    state.assemble_pitched_sound(
        Box::new(ADSR2.timed_moog(Box::new(ADSR2.timed_sound(Box::new(dsf_square()), state)), state)),
        ADSR2.boxed(state),
    )
}