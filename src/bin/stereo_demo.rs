use std::sync::Arc;

use crossbeam_queue::SegQueue;
use midi_fundsp::{
    get_first_midi_device, simple_triangle, start_input_thread, Speaker, SynthMsg, StereoSynth
};
use midi_msg::{ChannelVoiceMsg, MidiMsg};
use midir::MidiInput;

fn main() -> anyhow::Result<()> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    let in_port = get_first_midi_device(&mut midi_in)?;
    let midi_msgs = Arc::new(SegQueue::new());
    start_input_thread(midi_msgs.clone(), midi_in, in_port);
    let stereo_msgs = Arc::new(SegQueue::new());
    {
        let stereo_msgs = stereo_msgs.clone();
        std::thread::spawn(move || loop {
            if let Some(midi_msg) = midi_msgs.pop() {
                let new_speaker = side_from_pitch(&midi_msg); 
                let midi_msg = midi_msg.speaker_swapped(new_speaker);
                stereo_msgs.push(midi_msg);
            }
        });
    }
    let mut player = StereoSynth::<10>::stereo(Arc::new(simple_triangle), Arc::new(simple_triangle));
    player.run_output(stereo_msgs)?;
    Ok(())
}

fn side_from_pitch(midi_msg: &SynthMsg) -> Speaker {
    if let SynthMsg::Midi(midi_msg,_) = midi_msg {
    match midi_msg {
        MidiMsg::ChannelVoice { channel: _, msg } => match msg {
            ChannelVoiceMsg::NoteOn { note, velocity: _ }
            | ChannelVoiceMsg::NoteOff { note, velocity: _ } => if *note < 60 {
                return Speaker::Left;
            } else {
                return Speaker::Right;
            },
            _ => return Speaker::Both,
        },
        _ => return Speaker::Both,
    }
}
Speaker::Both
}
