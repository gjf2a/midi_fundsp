use std::sync::{Arc, Mutex};

use crossbeam_queue::SegQueue;
use crossbeam_utils::atomic::AtomicCell;
use midi_fundsp::{
    io::{get_first_midi_device, start_input_thread, Speaker, SynthMsg, start_output_thread},
    sounds::{moogs},
};
use midi_msg::{ChannelVoiceMsg, MidiMsg};
use midir::MidiInput;

fn main() -> anyhow::Result<()> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    let in_port = get_first_midi_device(&mut midi_in)?;
    let midi_msgs = Arc::new(SegQueue::new());
    let quit = Arc::new(AtomicCell::new(false));
    start_input_thread(midi_msgs.clone(), midi_in, in_port, quit.clone());
    let stereo_msgs = Arc::new(SegQueue::new());
    stereo_msgs.push(SynthMsg::program_change(1, Speaker::Left));
    let program_table = Arc::new(Mutex::new(moogs()));
    start_output_thread::<10>(stereo_msgs.clone(), program_table, quit);
    println!("Loops indefinitely, printing MIDI inputs as they arrive.\n\nUse CTRL-C to exit.");
    loop {
        if let Some(mut midi_msg) = midi_msgs.pop() {
            println!("{:?}", midi_msg.msg);
            midi_msg.speaker = side_from_pitch(&midi_msg);
            stereo_msgs.push(midi_msg);
        }
    }
}

fn side_from_pitch(midi_msg: &SynthMsg) -> Speaker {
    if let MidiMsg::ChannelVoice { channel: _, msg } = midi_msg.msg {
        match msg {
            ChannelVoiceMsg::NoteOn { note, velocity: _ }
            | ChannelVoiceMsg::NoteOff { note, velocity: _ } => {
                if note < 60 {
                    return Speaker::Left;
                } else {
                    return Speaker::Right;
                }
            }
            _ => {}
        }
    }
    Speaker::Both
}
