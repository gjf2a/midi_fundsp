use std::sync::Arc;

use crossbeam_queue::SegQueue;
use midi_fundsp::{
    io::{get_first_midi_device, start_input_thread, Speaker, StereoPlayer, SynthMsg},
    sounds::{options, simple_triangle},
};
use midir::MidiInput;
use read_input::prelude::*;

fn main() -> anyhow::Result<()> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    let in_port = get_first_midi_device(&mut midi_in)?;
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
            for i in 0..options.len() {
                println!("{}: {}", i + 1, options[i].0);
            }
            let choice = input()
                .msg("Change synth to: ")
                .inside(1..=options.len())
                .get();
            midi_msgs.push(SynthMsg::SetSynth(
                options[choice - 1].1.clone(),
                Speaker::Both,
            ));
        }
    });
}
