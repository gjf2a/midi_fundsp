use fundsp::hacker::{adsr_live, clamp01, envelope2, moog_q, xerp, AudioUnit64, Net64};

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
pub fn simple_sound(state: &SharedMidiState, synth: Box<dyn AudioUnit64>) -> Box<dyn AudioUnit64> {
    let control = state.control_var();
    state.assemble_unpitched_sound(
        synth,
        Box::new(control >> envelope2(move |_, n| clamp01(n))),
    )
}

#[derive(Copy, Clone, Debug)]
/// Represents ADSR (Attack/Decay/Sustain/Release) settings for the purpose of generating MIDI-ready sounds.
pub struct Adsr {
    pub attack: f64,
    pub decay: f64,
    pub sustain: f64,
    pub release: f64,
}

impl Adsr {
    /// Returns an ADSR filter in a `Box`.
    pub fn boxed(&self, state: &SharedMidiState) -> Box<dyn AudioUnit64> {
        let control = state.control_var();
        Box::new(control >> adsr_live(self.attack, self.decay, self.sustain, self.release))
    }

    /// Returns an ADSR filter in a `Net64`.
    pub fn net64ed(&self, state: &SharedMidiState) -> Net64 {
        Net64::wrap(self.boxed(state))
    }

    /// Stacks pitch with an ADSR and pipes them into `timed_sound`. Useful for any sound needing two 
    /// inputs, where the first is a pitch and the second is time-varying information.
    pub fn timed_sound(&self, timed_sound: Box<dyn AudioUnit64>, state: &SharedMidiState) -> Net64 {
        Net64::pipe_op(
            Net64::stack_op(state.bent_pitch(), self.net64ed(state)),
            Net64::wrap(timed_sound),
        )
    }

    /// Stacks `source` with an ADSR that is piped into an exponential interpolator.
    /// Thes two stacked inputs are then piped into a Moog filter.
    pub fn timed_moog(&self, source: Box<dyn AudioUnit64>, state: &SharedMidiState) -> Net64 {
        Net64::pipe_op(
            Net64::stack_op(
                Net64::wrap(source),
                Net64::pipe_op(
                    self.net64ed(state),
                    Net64::wrap(Box::new(envelope2(move |_, n| xerp(1100.0, 11000.0, n)))),
                ),
            ),
            Net64::wrap(Box::new(moog_q(0.6))),
        )
    }

    /// Convenience method to create a ready-to-go sound using `timed_sound()` above.
    pub fn assemble_timed(
        &self,
        timed_sound: Box<dyn AudioUnit64>,
        state: &SharedMidiState,
    ) -> Box<dyn AudioUnit64> {
        state.assemble_pitched_sound(
            Box::new(self.timed_sound(timed_sound, state)),
            self.boxed(state),
        )
    }
}
