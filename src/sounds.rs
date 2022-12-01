use fundsp::{
    hacker::{pulse, triangle, FrameMul, Net64},
    prelude::AudioUnit64,
};

use crate::{adsr_timed_moog, simple_sound, Adsr, SharedMidiState};

const ADSR1: Adsr = Adsr {
    attack: 0.1,
    decay: 0.2,
    sustain: 0.4,
    release: 0.4,
};

const ADSR2: Adsr = Adsr {
    attack: 0.1,
    decay: 0.4,
    sustain: 0.4,
    release: 0.6,
};

pub fn simple_triangle(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    simple_sound(state, Box::new(triangle()))
}

pub fn adsr_triangle(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    state.assemble_sound(Box::new(triangle()), ADSR1.boxed(state))
}

pub fn adsr_timed_pulse(state: &SharedMidiState, adsr: Adsr) -> Box<dyn AudioUnit64> {
    Box::new(Net64::bin_op(
        adsr.timed_sound(Box::new(pulse()), state),
        state.volume(adsr.boxed(state)),
        FrameMul::new(),
    ))
}

pub fn pulse1(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    adsr_timed_pulse(state, ADSR2)
}

pub fn triangle_moog(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    state.assemble_sound(
        adsr_timed_moog(state, Box::new(triangle()), ADSR2),
        ADSR2.boxed(state),
    )
}

pub fn pulse_moog(state: &SharedMidiState) -> Box<dyn AudioUnit64> {
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
