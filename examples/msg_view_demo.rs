use std::sync::{Arc, Mutex};

use crossbeam_queue::SegQueue;
use crossbeam_utils::atomic::AtomicCell;
use midi_fundsp::{
    io::{
        choose_midi_device, console_choice_from, start_input_thread, start_output_thread, Speaker,
        SynthMsg,
    },
    sound_builders::ProgramTable,
    sounds::options,
};
use midir::MidiInput;

fn main() -> anyhow::Result<()> {
    let reset = Arc::new(AtomicCell::new(false));
    let mut quit = false;
    while !quit {
        let mut midi_in = MidiInput::new("midir reading input")?;
        let in_port = choose_midi_device(&mut midi_in)?;
        let incoming_msgs = Arc::new(SegQueue::new());
        let outgoing_msgs = Arc::new(SegQueue::new());
        while reset.load() {}
        start_input_thread(incoming_msgs.clone(), midi_in, in_port, reset.clone());
        let program_table = Arc::new(Mutex::new(options()));
        start_output_thread::<10>(outgoing_msgs.clone(), program_table.clone());
        run_midi_show_thread(incoming_msgs, outgoing_msgs.clone());
        run_chooser(outgoing_msgs, program_table.clone(), reset.clone(), &mut quit);
    }
    Ok(())
}

fn run_midi_show_thread(incoming_msgs: Arc<SegQueue<SynthMsg>>, outgoing_msgs: Arc<SegQueue<SynthMsg>>) {
    std::thread::spawn(move || loop {
        if let Some(msg) = incoming_msgs.pop() {
            println!("{msg:?}");
            outgoing_msgs.push(msg);
        }
    });
}

fn run_chooser(
    midi_msgs: Arc<SegQueue<SynthMsg>>,
    program_table: Arc<Mutex<ProgramTable>>,
    reset: Arc<AtomicCell<bool>>,
    quit: &mut bool,
) {
    let main_menu = vec!["Pick New Synthesizer Sound", "Pick New MIDI Device", "Quit"];
    while !*quit && !reset.load() {
        println!("Play notes at will. When ready for a change, select one of the following:");
        match console_choice_from("Choice", &main_menu, |s| *s) {
            0 => {
                let program = {
                    let program_table = program_table.lock().unwrap();
                    console_choice_from("Change synth to", &program_table, |opt| opt.0.as_str())
                };
                midi_msgs.push(SynthMsg::program_change(program as u8, Speaker::Both));
            }
            1 => reset.store(true),
            2 => *quit = true,
            _ => panic!("This should never happen."),
        }
    }
}
