use anyhow::bail;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Sample, SampleFormat, StreamConfig};
use fundsp::hacker::{envelope, midi_hz, triangle, var};
use fundsp::prelude::{An, AudioUnit64, Tag, Var};
use midi_msg::{ChannelVoiceMsg, MidiMsg};
use midir::{Ignore, MidiInput, MidiInputPort};
use read_input::prelude::*;

const PITCH_TAG: Tag = 1;
const VELOCITY_TAG: Tag = PITCH_TAG + 1;

fn main() -> anyhow::Result<()> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    let in_port = get_midi_device(&mut midi_in)?;

    let pitch = var(PITCH_TAG, 0.0);
    let velocity = var(VELOCITY_TAG, 0.0);

    run_output(pitch.clone(), velocity.clone());
    run_input(pitch, velocity, midi_in, in_port)
}

fn run_input(
    outgoing_pitch: An<Var<f64>>,
    outgoing_velocity: An<Var<f64>>,
    midi_in: MidiInput,
    in_port: MidiInputPort,
) -> anyhow::Result<()> {
    println!("\nOpening connection");
    let in_port_name = midi_in.port_name(&in_port)?;
    let _conn_in = midi_in
        .connect(
            &in_port,
            "midir-read-input",
            move |_stamp, message, _| {
                let (msg, _len) = MidiMsg::from_midi(&message).unwrap();
                match msg {
                    MidiMsg::ChannelVoice { channel:_, msg } => {
                        match msg {
                            ChannelVoiceMsg::NoteOn { note, velocity } |
                            ChannelVoiceMsg::NoteOff { note, velocity } => {
                                outgoing_pitch.set_value(note as f64);
                                outgoing_velocity.set_value(velocity as f64);
                                println!("pitch: {} velocity: {}", outgoing_pitch.value(), outgoing_velocity.value());
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            },
            (),
        )
        .unwrap();
    println!("Connection open, reading input from '{in_port_name}'");

    let _ = input::<String>().msg("(press enter to exit)...\n").get();
    println!("Closing connection");
    Ok(())
}

fn get_midi_device(midi_in: &mut MidiInput) -> anyhow::Result<MidiInputPort> {
    midi_in.ignore(Ignore::None);
    let in_ports = midi_in.ports();
    if in_ports.len() == 0 {
        bail!("No MIDI devices attached")
    } else {
        println!(
            "Chose MIDI device {}",
            midi_in.port_name(&in_ports[0]).unwrap()
        );
        Ok(in_ports[0].clone())
    }
}

fn run_output(incoming_pitch: An<Var<f64>>, incoming_velocity: An<Var<f64>>) {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config().unwrap();
    match config.sample_format() {
        SampleFormat::F32 => run_synth::<f32>(incoming_pitch, incoming_velocity, device, config.into()),
        SampleFormat::I16 => run_synth::<i16>(incoming_pitch, incoming_velocity, device, config.into()),
        SampleFormat::U16 => run_synth::<u16>(incoming_pitch, incoming_velocity, device, config.into()),
    }
}

fn run_synth<T: Sample>(
    incoming_pitch: An<Var<f64>>,
    incoming_velocity: An<Var<f64>>,
    device: Device,
    config: StreamConfig,
) {
    let sample_rate = config.sample_rate.0 as f64;
    let mut sound = create_sound(incoming_pitch, incoming_velocity);
    sound.reset(Some(sample_rate));
    let mut next_value = move || sound.get_stereo();
    let channels = config.channels as usize;
    std::thread::spawn(move || {
        let err_fn = |err| eprintln!("an error occurred on stream: {err}");
        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    write_data(data, channels, &mut next_value)
                },
                err_fn,
            )
            .unwrap();

        stream.play().unwrap();
        loop {}
    });
}

fn create_sound(incoming_pitch: An<Var<f64>>, incoming_velocity: An<Var<f64>>) -> Box<dyn AudioUnit64> {
    Box::new(
        envelope(move |_t| midi_hz(incoming_pitch.value())) >> triangle() * envelope(move |_t| (incoming_velocity.value() / 127.0)),
    )
}

fn write_data<T: Sample>(
    output: &mut [T],
    channels: usize,
    next_sample: &mut dyn FnMut() -> (f64, f64),
) {
    for frame in output.chunks_mut(channels) {
        let sample = next_sample();
        let left: T = Sample::from::<f32>(&(sample.0 as f32));
        let right: T = Sample::from::<f32>(&(sample.1 as f32));

        for (channel, sample) in frame.iter_mut().enumerate() {
            *sample = if channel & 1 == 0 { left } else { right };
        }
    }
}
