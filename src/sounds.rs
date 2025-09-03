use std::sync::Arc;

use crate::sound_builders::{simple_sound, Adsr, ProgramTable};
use crate::{program_table, SharedMidiState};
use fundsp::hacker::{
    dsf_saw, dsf_square, organ, pulse, saw, sine, soft_saw, square, triangle, AudioUnit,
};
use fundsp::hacker32::highpass_hz;
use fundsp::prelude::{
    brown, db_amp, dcblock, highshelf_hz, limiter, lowpass_hz,
    mul, pass, resonator_hz,
};

/// Returns a `ProgramTable` containing all prepared sounds in this file.
pub fn options() -> ProgramTable {
    program_table![
        ("Acoustic Grand Piano", acoustic_grand_piano),
        ("Simple Triangle", simple_triangle),
        ("Triangle", adsr_triangle),
        ("Organ", adsr_organ),
        ("Sine", adsr_sine),
        ("Saw", adsr_saw),
        ("Soft Saw", adsr_soft_saw),
        ("Square", adsr_square),
        ("Pulse", adsr_pulse),
        ("DSF Saw", adsr_dsf_saw),
        ("DSF Square", adsr_dsf_square),
        ("Moog Organ", moog_organ),
        ("Moog Saw", moog_saw),
        ("Moog Soft Saw", moog_soft_saw),
        ("Moog Square", moog_square),
        ("Moog Pulse", moog_pulse)
    ]
}

/// Returns a `ProgramTable` containing sounds that are personal favorites of the crate author.
pub fn favorites() -> ProgramTable {
    program_table![
        ("80s Beep", simple_triangle),
        ("Triangle", adsr_triangle),
        ("Organ", adsr_organ),
        ("Saw", adsr_saw),
        ("Soft Saw", adsr_soft_saw),
        ("Square", adsr_square),
        ("Pulse", adsr_pulse),
        ("Moog Organ", moog_organ),
        ("Moog Saw", moog_saw),
        ("Moog Square", moog_square),
        ("Moog Pulse", moog_pulse)
    ]
}

/// Returns a `ProgramTable` containing Moog sounds.
pub fn moogs() -> ProgramTable {
    program_table![
        ("Moog Organ", moog_organ),
        ("Moog Pulse", moog_pulse),
        ("Moog Saw", moog_saw),
        ("Moog Square", moog_square)
    ]
}

/// Returns an on-off Triangle wave.
pub fn simple_triangle(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    simple_sound(state, Box::new(triangle() * 4.4))
}

/// ADSR envelope used in some sounds.
pub const ADSR1: Adsr = Adsr {
    attack: 0.1,
    decay: 0.2,
    sustain: 0.4,
    release: 0.4,
};

/// ADSR envelope used in sounds benefiting from a long decay and release.
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

/// Triangle wave modulated by an ADSR.
pub fn adsr_triangle(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    state.assemble_unpitched_sound(Box::new(triangle() * 4.4), ADSR1.boxed(state))
}

/// Organ wave modulated by an ADSR.
pub fn adsr_organ(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    state.assemble_unpitched_sound(Box::new(organ() * 3.45), ADSR1.boxed(state))
}

/// Sine wave modulated by an ADSR.
pub fn adsr_sine(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    state.assemble_unpitched_sound(Box::new(sine()), ADSR1.boxed(state))
}

/// Sawtooth wave modulated by an ADSR.
pub fn adsr_saw(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    state.assemble_unpitched_sound(Box::new(saw() * 2.3), ADSR1.boxed(state))
}

/// Soft sawtooth wave modulated by an ADSR.
pub fn adsr_soft_saw(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    state.assemble_unpitched_sound(Box::new(soft_saw() * 4.0), ADSR1.boxed(state))
}

/// Square wave modulated by an ADSR.
pub fn adsr_square(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    state.assemble_unpitched_sound(Box::new(square() * 2.5), ADSR1.boxed(state))
}

/// Pulse wave modulated by an ADSR.
pub fn adsr_pulse(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    ADSR2.assemble_timed(Box::new(pulse() * 2.8), state)
}

/// DSF Sawtooth wave modulated by an ADSR.
pub fn adsr_dsf_saw(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    ADSR2.assemble_timed(Box::new(dsf_saw() * 0.08), state)
}

/// DSF Square wave modulated by an ADSR.
pub fn adsr_dsf_square(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    ADSR2.assemble_timed(Box::new(dsf_square() * 0.08), state)
}

/// Pulse wave through a Moog filter modulated by an ADSR.
pub fn moog_pulse(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    state.assemble_pitched_sound(
        Box::new(ADSR2.timed_moog(
            Box::new(ADSR2.timed_sound(Box::new(pulse() * 4.5), state)),
            state,
        )),
        ADSR2.boxed(state),
    )
}

/// Square wave through a Moog filter modulated by an ADSR.
pub fn moog_square(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    state.assemble_unpitched_sound(
        Box::new(ADSR2.timed_moog(Box::new(square() * 5.625), state)),
        ADSR2.boxed(state),
    )
}

/// Sawtooth wave through a Moog filter modulated by an ADSR.
pub fn moog_saw(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    state.assemble_unpitched_sound(
        Box::new(ADSR2.timed_moog(Box::new(saw() * 5.0), state)),
        ADSR2.boxed(state),
    )
}

/// Sawtooth wave through a Moog filter modulated by an ADSR.
pub fn moog_soft_saw(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    state.assemble_unpitched_sound(
        Box::new(ADSR2.timed_moog(Box::new(soft_saw() * 7.6), state)),
        ADSR2.boxed(state),
    )
}

/// Organ wave through a Moog filtered modulated by an ADSR.
pub fn moog_organ(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    state.assemble_unpitched_sound(
        Box::new(ADSR2.timed_moog(Box::new(organ() * 6.7), state)),
        ADSR2.boxed(state),
    )
}

pub fn acoustic_grand_piano(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    let piano_adsr = Adsr {
        attack: 0.002,
        decay: 0.8,
        sustain: 0.2,
        release: 0.7,
    };

    let a1 = 0.9;
    let a2 = 0.55;
    let a3 = 0.42;
    let a4 = 0.38;
    let a5 = 0.25;
    let a6 = 0.13;
    let a7 = 0.02;
    let a8 = 0.015;
    let a9 = 0.01;

    let tone = (mul(1.0) >> sine() * a1)
        & (mul(2.0) >> sine() * a2)
        & (mul(3.0) >> sine() * a3)
        & (mul(4.0) >> sine() * a4)
        & (mul(5.0) >> sine() * a5)
        & (mul(6.0) >> sine() * a6)
        & (mul(7.0) >> sine() * a7)
        & (mul(8.0) >> sine() * a8)
        & (mul(9.0) >> sine() * a9)
            >> lowpass_hz(400.0, 0.8)
            >> highshelf_hz(2000.0, db_amp(-12.0), 0.0)
            >> dcblock::<f64>();

    let body = pass()
        & (0.25 * resonator_hz(82.4, 25.0))
        & (0.20 * resonator_hz(110.0, 22.0))
        & (0.18 * resonator_hz(146.8, 20.0))
        & (0.15 * resonator_hz(220.0, 18.0))
        & (0.12 * resonator_hz(293.7, 16.0))
        & (0.10 * resonator_hz(440.0, 14.0))
        & (0.08 * resonator_hz(587.3, 12.0))
        & (0.06 * resonator_hz(880.0, 10.0))
        & (0.04 * resonator_hz(1760.0, 8.0))
        & (0.02 * resonator_hz::<f64>(800.0, 10.0) * brown::<f64>());

    let synth = Box::new(
        tone >> body >> highpass_hz(20.0, 0.7) >> limiter(0.002, 0.06) >> dcblock::<f64>(),
    );

    state.assemble_unpitched_sound(synth, piano_adsr.boxed(state))
}
