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
    let mut midi_in = MidiInput::new("midir reading input")?;
    let in_port = choose_midi_device(&mut midi_in)?;
    let midi_msgs = Arc::new(SegQueue::new());
    let quit = Arc::new(AtomicCell::new(false));
    start_input_thread(midi_msgs.clone(), midi_in, in_port, quit.clone(), false);
    start_chooser_thread(midi_msgs.clone(), quit.clone());
    let mut player = StereoPlayer::<10>::mono(Arc::new(simple_triangle));
    player.run_output(midi_msgs, quit)?;
    Ok(())
}

fn start_chooser_thread(midi_msgs: Arc<SegQueue<SynthMsg>>, quit: Arc<AtomicCell<bool>>) {
    std::thread::spawn(move || {
        let main_menu = vec!["Pick New Synthesizer Sound", "Quit"];
        let options = options();
        while !quit.load() {
            let choice = console_choice_from("Choice", &main_menu, |s| *s, |s| *s);
            if choice == "Quit" {
                quit.store(true);
            } else {
                let synth = console_choice_from(
                    "Change synth to",
                    &options,
                    |opt| opt.0,
                    |opt| opt.1.clone(),
                );
                midi_msgs.push(SynthMsg::SetSynth(synth.clone(), Speaker::Both));
            }
        }
    });
}
