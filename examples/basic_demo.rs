use std::sync::Arc;

use crossbeam_queue::SegQueue;
use crossbeam_utils::atomic::AtomicCell;
use midi_fundsp::{
    io::{get_first_midi_device, start_input_thread, StereoPlayer},
    sounds::simple_triangle,
};
use midir::MidiInput;

fn main() -> anyhow::Result<()> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    let in_port = get_first_midi_device(&mut midi_in)?;
    let midi_msgs = Arc::new(SegQueue::new());
    let quit = Arc::new(AtomicCell::new(false));
    start_input_thread(midi_msgs.clone(), midi_in, in_port, quit.clone(), true);
    let mut player = StereoPlayer::<10>::mono(Arc::new(simple_triangle));
    player.run_output(midi_msgs, quit)?;
    Ok(())
}
