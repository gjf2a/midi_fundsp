use std::sync::Arc;

use crossbeam_queue::SegQueue;
use midi_fundsp::{
    io::{
        choose_midi_device, console_choice_from, start_input_thread, Speaker, StereoPlayer,
        SynthMsg,
    },
    sounds::{options, simple_triangle},
};
use midir::MidiInput;

fn main() -> anyhow::Result<()> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    let in_port = choose_midi_device(&mut midi_in)?;
    let midi_msgs = Arc::new(SegQueue::new());
    start_input_thread(midi_msgs.clone(), midi_in, in_port);
    start_chooser_thread(midi_msgs.clone());
    let mut player = StereoPlayer::<10>::mono(Arc::new(simple_triangle));
    player.run_output(midi_msgs)?;
    Ok(())
}

fn start_chooser_thread(midi_msgs: Arc<SegQueue<SynthMsg>>) {
    std::thread::spawn(move || {
        let options = options();
        loop {
            let synth = console_choice_from(
                "Change synth to",
                &options,
                |opt| opt.0,
                |opt| opt.1.clone(),
            );
            midi_msgs.push(SynthMsg::SetSynth(synth.clone(), Speaker::Both));
        }
    });
}
