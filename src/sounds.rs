use std::sync::Arc;

use fundsp::hacker::{pulse, sine, triangle, AudioUnit64, FrameMul, Net64};

use crate::{adsr_timed_moog, simple_sound, Adsr, SharedMidiState, SynthFunc};

pub fn options() -> Vec<(&'static str, SynthFunc)> {
    vec![
        ("Simple Triangle", Arc::new(simple_triangle)),
        ("Triangle", Arc::new(adsr_triangle)),
        ("Sine", Arc::new(adsr_sine)),
        ("Pulse", Arc::new(adsr_pulse)),
        ("Moog Triangle", Arc::new(moog_triangle)),
        ("Moog Pulse", Arc::new(moog_pulse)),
        //("Pluck", Arc::new(adsr_pluck))
    ]
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

pub fn simple_triangle(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    simple_sound(state, Box::new(triangle()))
}

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

pub fn adsr_timed_pulse(state: &SharedMidiState, adsr: Adsr) -> Box<dyn AudioUnit64> {
    Box::new(Net64::bin_op(
        adsr.timed_sound(Box::new(pulse()), state),
        state.volume(adsr.boxed(state)),
        FrameMul::new(),
    ))
}

pub fn adsr_pulse(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    adsr_timed_pulse(state, ADSR2)
}

pub fn moog_triangle(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    state.assemble_sound(
        adsr_timed_moog(state, Box::new(triangle()), ADSR2),
        ADSR2.boxed(state),
    )
}

pub fn moog_sine(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    state.assemble_sound(
        adsr_timed_moog(state, Box::new(sine()), ADSR2),
        ADSR2.boxed(state),
    )
}

pub fn moog_pulse(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    Box::new(Net64::bin_op(
        Net64::wrap(adsr_timed_moog(
            state,
            Box::new(ADSR2.timed_sound(Box::new(pulse()), state)),
            ADSR2,
        )),
        state.volume(ADSR2.boxed(state)),
        FrameMul::new(),
    ))
}
