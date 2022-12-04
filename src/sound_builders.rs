use fundsp::hacker::{adsr_live, clamp01, envelope2, moog_q, xerp, AudioUnit64, Net64};

use crate::{SharedMidiState, SynthFunc};

pub const NUM_PROGRAM_SLOTS: usize = 2_usize.pow(7);
pub type ProgramTable = Vec<(String, SynthFunc)>;

pub fn simple_sound(state: &SharedMidiState, synth: Box<dyn AudioUnit64>) -> Box<dyn AudioUnit64> {
    let control = state.control_var();
    state.assemble_sound(
        synth,
        Box::new(control >> envelope2(move |_, n| clamp01(n))),
    )
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
