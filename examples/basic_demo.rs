use std::sync::{Arc, Mutex};

use crossbeam_queue::SegQueue;
use crossbeam_utils::atomic::AtomicCell;
use midi_fundsp::{
    io::{get_first_midi_device, start_input_thread, start_output_thread},
    sounds::options,
};
use midir::MidiInput;
use read_input::{InputBuild, shortcut::input};

fn main() -> anyhow::Result<()> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    let in_port = get_first_midi_device(&mut midi_in)?;
    let midi_msgs = Arc::new(SegQueue::new());
    let quit = Arc::new(AtomicCell::new(false));
    start_input_thread(midi_msgs.clone(), midi_in, in_port, quit.clone());
    start_output_thread::<10>(midi_msgs, Arc::new(Mutex::new(options())));
    input::<String>().msg("Press any key to exit\n").get();
    Ok(())
}
