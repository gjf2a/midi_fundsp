use std::sync::Arc;

use crossbeam_queue::SegQueue;
use midi_fundsp::{
    get_first_midi_device, simple_triangle, start_input_thread, Player, StereoMsg, StereoSide,
    StereoSounds,
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
                if let Some(side) = side_from_pitch(&midi_msg) {
                    stereo_msgs.push(StereoMsg { midi_msg, side });
                } else {
                    stereo_msgs.push(StereoMsg {
                        midi_msg: midi_msg.clone(),
                        side: StereoSide::Left,
                    });
                    stereo_msgs.push(StereoMsg {
                        midi_msg,
                        side: StereoSide::Right,
                    });
                }
            }
        });
    }
    let mut player = StereoSounds::<10>::new(Arc::new(simple_triangle), Arc::new(simple_triangle));
    player.run_output(stereo_msgs)?;
    Ok(())
}

fn side_from_pitch(midi_msg: &MidiMsg) -> Option<StereoSide> {
    match midi_msg {
        MidiMsg::ChannelVoice { channel: _, msg } => match msg {
            ChannelVoiceMsg::NoteOn { note, velocity: _ }
            | ChannelVoiceMsg::NoteOff { note, velocity: _ } => Some(if *note < 60 {
                StereoSide::Left
            } else {
                StereoSide::Right
            }),
            _ => None,
        },
        _ => None,
    }
}
