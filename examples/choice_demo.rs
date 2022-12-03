use std::sync::Arc;

use crossbeam_queue::SegQueue;
use crossbeam_utils::atomic::AtomicCell;
use midi_fundsp::{
    io::{
        choose_midi_device, console_choice_from, start_input_thread, Speaker, StereoPlayer,
        SynthMsg,
    },
    sounds::{options, simple_triangle},
};
use midir::MidiInput;

fn main() -> anyhow::Result<()> {
    let reset = Arc::new(AtomicCell::new(false));
    let quit = Arc::new(AtomicCell::new(false));
    while !quit.load() {
        let mut midi_in = MidiInput::new("midir reading input")?;
        let in_port = choose_midi_device(&mut midi_in)?;
        let midi_msgs = Arc::new(SegQueue::new());
        start_input_thread(midi_msgs.clone(), midi_in, in_port, reset.clone(), false);
        start_output_thread(midi_msgs.clone(), reset.clone());
        run_chooser(midi_msgs, reset.clone(), quit.clone());
    }
    Ok(())
}

fn start_output_thread(midi_msgs: Arc<SegQueue<SynthMsg>>, quit: Arc<AtomicCell<bool>>) {
    std::thread::spawn(move || {
        let mut player = StereoPlayer::<10>::mono(Arc::new(simple_triangle));
        player.run_output(midi_msgs, quit).unwrap();
    });
}

fn run_chooser(midi_msgs: Arc<SegQueue<SynthMsg>>, reset: Arc<AtomicCell<bool>>, quit: Arc<AtomicCell<bool>>) {
    let main_menu = vec!["Pick New Synthesizer Sound", "Pick New MIDI Device", "Quit"];
    let options = options();
    reset.store(false);
    while !quit.load() && !reset.load() {
        match console_choice_from("Choice", &main_menu, |s| *s) {
            0 => {
                let synth = console_choice_from(
                    "Change synth to",
                    &options,
                    |opt| opt.0,
                );
                midi_msgs.push(SynthMsg::SetSynth(options[synth].1.clone(), Speaker::Both));
            }
            1 => reset.store(true),
            2 => quit.store(true),
            _ => panic!("This should never happen.")
        }
    }
}
