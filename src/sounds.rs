use fundsp::{
    hacker::{
        adsr_live, clamp01, envelope, envelope2, lerp11, pulse, sin_hz, triangle, FrameMul, Net64,
    },
    prelude::AudioUnit64,
};

use crate::io::SharedMidiState;

pub type Adsr = (f64, f64, f64, f64);

pub fn simple_triangle(shared_midi_state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    simple_sound(shared_midi_state, Box::new(triangle()))
}

pub fn simple_sound(
    shared_midi_state: &SharedMidiState,
    synth: Box<dyn AudioUnit64>,
) -> Box<dyn AudioUnit64> {
    let control = shared_midi_state.control_var();
    shared_midi_state.assemble_sound(synth, Box::new(envelope(move |_| clamp01(control.value()))))
}

pub fn adsr_triangle(shared_midi_state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    adsr_sound(
        shared_midi_state,
        Box::new(triangle()),
        (0.1, 0.2, 0.4, 0.4),
    )
}

pub fn adsr_sound(
    shared_midi_state: &SharedMidiState,
    synth: Box<dyn AudioUnit64>,
    adsr: Adsr,
) -> Box<dyn AudioUnit64> {
    let control = shared_midi_state.control_var();
    let (attack, decay, sustain, release) = adsr;
    shared_midi_state.assemble_sound(
        synth,
        Box::new(control >> adsr_live(attack, decay, sustain, release)),
    )
}

pub fn adsr_timed_sound(
    shared_midi_state: &SharedMidiState,
    adsr: Adsr,
    synth: Box<dyn AudioUnit64>,
) -> Box<dyn AudioUnit64> {
    let (attack, decay, sustain, release) = adsr;
    let control1 = shared_midi_state.control_var();
    let control2 = control1.clone();
    Box::new(Net64::bin_op(
        Net64::pipe_op(
            Net64::stack_op(
                shared_midi_state.bent_pitch(),
                Net64::wrap(Box::new(
                    control1 >> adsr_live(attack, decay, sustain, release),
                )),
            ),
            Net64::wrap(synth),
        ),
        shared_midi_state.volume(Box::new(
            control2 >> adsr_live(attack, decay, sustain, release),
        )),
        FrameMul::new(),
    ))
}

pub fn adsr_timed_pulse(shared_midi_state: &SharedMidiState, adsr: Adsr) -> Box<dyn AudioUnit64> {
    adsr_timed_sound(shared_midi_state, adsr, Box::new(pulse()))
}

/*pub fn adsr_timed_moog(shared_midi_state: &SharedMidiState, base: Box<dyn AudioUnit64>, adsr: Adsr) -> Box<dyn AudioUnit64> {
    adsr_timed_sound(shared_midi_state, adsr, Box::new(Net64::stack_op(base, net2)))
}*/

pub fn pulse1(shared_midi_state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    adsr_timed_pulse(shared_midi_state, (0.1, 0.2, 0.4, 0.4))
}

pub fn pulse2(shared_midi_state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    adsr_sound(
        shared_midi_state,
        Box::new(envelope2(move |t, p| (p, lerp11(0.01, 0.99, sin_hz(0.05, t)))) >> pulse()),
        (0.1, 0.2, 0.4, 0.4),
    )
}
