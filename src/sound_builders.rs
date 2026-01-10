use fundsp::{
    math::{clamp01, xerp},
    net::Net,
    prelude::AudioUnit,
    prelude64::{adsr_live, envelope2, moog_q},
};

use crate::{SharedMidiState, SynthFunc};

#[macro_export]
/// Convenience macro to build a `ProgramTable`. Given a sequence of tuples of `&str` objects
/// and `SynthFunc` objects, it returns a proper `ProgramTable`.
macro_rules! program_table {
    ($( ($s:expr, $f:expr)),* ) => {vec![$(($s.to_owned(), Arc::new($f)),)*]}
}

/// Maximum number of entries controllable via MIDI messages in a MIDI program table.
pub const NUM_PROGRAM_SLOTS: usize = 2_usize.pow(7);

/// Convenience type alias for MIDI program tables.
pub type ProgramTable = Vec<(String, SynthFunc)>;

/// Pipes a pitch into `synth`, then modulates the output volume depending on MIDI status.
pub fn simple_sound(state: &SharedMidiState, synth: Box<dyn AudioUnit>) -> Box<dyn AudioUnit> {
    let control = state.control_var();
    state.assemble_unpitched_sound(
        synth,
        Box::new(control >> envelope2(move |_, n| clamp01(n))),
    )
}

#[derive(Copy, Clone, Debug)]
/// Represents ADSR (Attack/Decay/Sustain/Release) settings for the purpose of generating MIDI-ready sounds.
pub struct Adsr {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Adsr {
    /// Returns an ADSR filter in a `Box`.
    pub fn boxed(&self, state: &SharedMidiState) -> Box<dyn AudioUnit> {
        let control = state.control_var();
        Box::new(control >> adsr_live(self.attack, self.decay, self.sustain, self.release))
    }

    /// Returns an ADSR filter in a `Net64`.
    pub fn net64ed(&self, state: &SharedMidiState) -> Net {
        Net::wrap(self.boxed(state))
    }

    /// Stacks pitch with an ADSR and pipes them into `timed_sound`. Useful for any sound needing two
    /// inputs, where the first is a pitch and the second is time-varying information.
    pub fn timed_sound(&self, timed_sound: Box<dyn AudioUnit>, state: &SharedMidiState) -> Net {
        Net::pipe(
            Net::stack(state.bent_pitch(), self.net64ed(state)),
            Net::wrap(timed_sound),
        )
    }

    /// Stacks `source` with an ADSR that is piped into an exponential interpolator.
    /// Thes two stacked inputs are then piped into a Moog filter.
    pub fn timed_moog(&self, source: Box<dyn AudioUnit>, state: &SharedMidiState) -> Net {
        Net::pipe(
            Net::stack(
                Net::wrap(source),
                Net::pipe(
                    self.net64ed(state),
                    Net::wrap(Box::new(envelope2(move |_, n| xerp(1100.0, 11000.0, n)))),
                ),
            ),
            Net::wrap(Box::new(moog_q(0.6))),
        )
    }

    /// Convenience method to create a ready-to-go sound using `timed_sound()` above.
    pub fn assemble_timed(
        &self,
        timed_sound: Box<dyn AudioUnit>,
        state: &SharedMidiState,
    ) -> Box<dyn AudioUnit> {
        state.assemble_pitched_sound(
            Box::new(self.timed_sound(timed_sound, state)),
            self.boxed(state),
        )
    }
}
