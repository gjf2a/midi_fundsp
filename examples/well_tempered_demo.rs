use std::sync::{Arc, Mutex};

use crossbeam_queue::SegQueue;
use crossbeam_utils::atomic::AtomicCell;
use midi_fundsp::{
    io::{get_first_midi_device, start_midi_input_thread, start_midi_output_thread_alt_tuning},
    sounds::options,
    tunings::well_temperament,
};
use midir::MidiInput;
use read_input::{InputBuild, shortcut::input};

fn main() -> anyhow::Result<()> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    let in_port = get_first_midi_device(&mut midi_in)?;
    let midi_msgs = Arc::new(SegQueue::new());
    let quit = Arc::new(AtomicCell::new(false));
    start_midi_input_thread(midi_msgs.clone(), midi_in, in_port, quit.clone());
    start_midi_output_thread_alt_tuning::<10>(
        midi_msgs,
        Arc::new(Mutex::new(options())),
        well_temperament,
    );
    input::<String>().msg("Press any key to exit\n").get();
    Ok(())
}
