use fundsp::{
    hacker::{
        adsr_live, clamp01, envelope, envelope2, moog_q, pulse, triangle, xerp, FrameMul, Net64,
    },
    prelude::AudioUnit64,
};

use crate::io::SharedMidiState;

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

pub fn simple_sound(state: &SharedMidiState, synth: Box<dyn AudioUnit64>) -> Box<dyn AudioUnit64> {
    let control = state.control_var();
    state.assemble_sound(synth, Box::new(envelope(move |_| clamp01(control.value()))))
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
                Net64::wrap(Box::new(envelope2(|_, n| xerp(1100.0, 11000.0, n)))),
            ),
        ),
        Net64::wrap(Box::new(moog_q(0.6))),
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
