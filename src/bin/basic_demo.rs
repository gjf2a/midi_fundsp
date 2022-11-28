use std::sync::Arc;

use crossbeam_queue::SegQueue;
use midi_fundsp::{get_first_midi_device, start_input_thread, LiveSounds, simple_triangle, run_output};
use midir::MidiInput;

fn main() -> anyhow::Result<()> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    let in_port = get_first_midi_device(&mut midi_in)?;
    let midi_msgs = Arc::new(SegQueue::new());
    start_input_thread(midi_msgs.clone(), midi_in, in_port);
    let player = LiveSounds::<10>::new(Arc::new(simple_triangle));
    run_output(player, midi_msgs);
    Ok(())
}