use std::sync::{Arc, Mutex};

use crossbeam_queue::SegQueue;
use crossbeam_utils::atomic::AtomicCell;
use midi_fundsp::{
    io::{get_first_midi_device, start_midi_input_thread, start_output_thread, Speaker, SynthMsg},
    program_table,
    sound_builders::ProgramTable,
    sounds::{adsr_pulse, moog_pulse},
};
use midi_msg::MidiMsg;
use midir::MidiInput;
use read_input::{shortcut::input, InputBuild};

fn main() -> anyhow::Result<()> {
    let stereo_table = Arc::new(Mutex::new(stereo_table()));

    let mut midi_in = MidiInput::new("midir reading input")?;
    let in_port = get_first_midi_device(&mut midi_in)?;
    let midi_msgs = Arc::new(SegQueue::new());
    let quit = Arc::new(AtomicCell::new(false));

    start_midi_input_thread(midi_msgs.clone(), midi_in, in_port, quit.clone());
    let stereo_msgs = Arc::new(SegQueue::new());
    stereo_msgs.push(SynthMsg::program_change(1, Speaker::Left));
    start_output_thread::<10>(stereo_msgs.clone(), stereo_table);

    println!("Play notes at will.");
    println!("Notes below middle C will be played on the left speaker with a pulse wave through a Moog filter.");
    println!("Notes at middle C or above will be played on the right speaker with a pulse wave.");
    println!("Loops indefinitely, printing MIDI inputs as they arrive.");

    std::thread::spawn(move || loop {
        if let Some(msg) = midi_msgs.pop() {
            stereo_msgs.push(msg_with_speaker(msg));
        }
    });

    input::<String>().msg("Press any key to exit\n").get();
    Ok(())
}

fn msg_with_speaker(msg: MidiMsg) -> SynthMsg {
    let mut result = SynthMsg {
        msg,
        speaker: Speaker::Both,
    };
    if let Some((note, _)) = result.note_velocity() {
        result.speaker = if note < 60 {
            Speaker::Left
        } else {
            Speaker::Right
        };
    }
    result
}

fn stereo_table() -> ProgramTable {
    program_table![("Pulse", adsr_pulse), ("Moog Pulse", moog_pulse)]
}
