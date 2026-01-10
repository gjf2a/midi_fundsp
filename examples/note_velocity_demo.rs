use std::sync::{Arc, Mutex};

use crossbeam_queue::SegQueue;
use crossbeam_utils::atomic::AtomicCell;
use midi_fundsp::{
    io::{choose_midi_device, start_input_thread, start_output_thread},
    sounds::options,
};
use midir::MidiInput;
use read_input::{InputBuild, shortcut::input};

fn main() -> anyhow::Result<()> {
    let reset = Arc::new(AtomicCell::new(false));

    let mut midi_in = MidiInput::new("midir reading input")?;
    let in_port = choose_midi_device(&mut midi_in)?;
    let inputs = Arc::new(SegQueue::new());
    let outputs = Arc::new(SegQueue::new());
    start_input_thread(inputs.clone(), midi_in, in_port, reset.clone());
    let program_table = Arc::new(Mutex::new(options()));
    start_output_thread::<10>(outputs.clone(), program_table.clone());
    std::thread::spawn(move || {
        loop {
            if let Some(msg) = inputs.pop() {
                if let Some((note, velocity)) = msg.note_velocity() {
                    println!("note: {note} velocity: {velocity}");
                }
                outputs.push(msg);
            }
        }
    });
    input::<String>().msg("Press any key to exit\n").get();
    Ok(())
}
