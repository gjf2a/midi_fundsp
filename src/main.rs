use std::collections::BTreeMap;
use anyhow::bail;
use bare_metal_modulo::{MNum, ModNumC};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Sample, SampleFormat, StreamConfig};
use fundsp::hacker::{AudioNode, envelope, midi_hz, triangle, var};
use fundsp::prelude::{An, AudioUnit64, Tag, Var};
use midi_msg::{ChannelVoiceMsg, MidiMsg};
use midir::{Ignore, MidiInput, MidiInputPort};
use read_input::prelude::*;

const PITCH_TAG: Tag = 1;
const VELOCITY_TAG: Tag = PITCH_TAG + 1;

fn main() -> anyhow::Result<()> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    let in_port = get_midi_device(&mut midi_in)?;

    let pv: PitchVelocities<8> = PitchVelocities::new();

    run_output(pv.clone());
    run_input(pv, midi_in, in_port)
}

macro_rules! envelop {
    ($a:expr, $pv:ident, $i:expr) => {
        {
            let pitch = $pv.pitches[$i].clone();
            let velocity = $pv.velocities[$i].clone();
            envelope(move |_| midi_hz(pitch.value())) >> $a * envelope(move |_| velocity.value() / 127.0)
        }
    }
}

#[derive(Clone)]
pub struct PitchVelocities<const N: usize> {
    pitches: [An<Var<f64>>; N],
    velocities: [An<Var<f64>>; N],
    pitch2entry: BTreeMap<u8,usize>,
    next_available: ModNumC<usize, N>
}

impl <const N: usize> PitchVelocities<N> {
    pub fn new() -> Self {
        PitchVelocities {
            // Array initialization: https://stackoverflow.com/a/69756635/906268
            pitches:  [(); N].map(|_| var(PITCH_TAG, 0.0)),
            velocities:  [(); N].map(|_| var(VELOCITY_TAG, 0.0)),
            pitch2entry: BTreeMap::new(),
            next_available: ModNumC::new(0)
        }
    }

    pub fn on(&mut self, note: u8, velocity: u8) {
        self.pitches[self.next_available.a()].clone().set_value(note as f64);
        self.velocities[self.next_available.a()].clone().set_value(velocity as f64);
        self.pitch2entry.insert(note, self.next_available.a());
        self.next_available += 1;
    }

    pub fn off(&mut self, note: u8) {
        if let Some(index) = self.pitch2entry.remove(&note) {
            self.velocities[index].clone().set_value(0.0);
        }
    }
/*
    pub fn create_sound(&self) -> impl AudioNode {
        self.create_sound_help(self.pitches.len() - 1)
    }

    fn create_sound_help(&self, index: usize) -> impl AudioNode {
        let e = envelop!(triangle(), self, index);
        if index == 0 {
            e
        } else {
            self.create_sound_help(index - 1) + e
        }
    }

 */
}

fn run_input<const N: usize>(
    mut pv: PitchVelocities<N>,
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
                            ChannelVoiceMsg::NoteOn { note, velocity } => {
                                pv.on(note, velocity);
                            }
                            ChannelVoiceMsg::NoteOff { note, velocity:_ } => {
                                pv.off(note)
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

fn run_output<const N: usize>(pv: PitchVelocities<N>) {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config().unwrap();
    match config.sample_format() {
        SampleFormat::F32 => run_synth::<f32,N>(pv, device, config.into()),
        SampleFormat::I16 => run_synth::<i16,N>(pv, device, config.into()),
        SampleFormat::U16 => run_synth::<u16,N>(pv, device, config.into()),
    }
}

// {vec![$(($s, Arc::new($f)),)*]}
macro_rules! sum_sound {
    ($var:ident, $($s:expr),* ) => {
        ($( envelop!(triangle(), $var, $s) + )*)
    }
}

fn run_synth<T: Sample, const N: usize>(
    pv: PitchVelocities<N>,
    device: Device,
    config: StreamConfig,
) {
    std::thread::spawn(move || {
        let sample_rate = config.sample_rate.0 as f64;
        //let mut sound = envelop!(triangle(), pv, 0) | envelop!(triangle(), pv, 1);
        //let mut sound = sum_sound!(pv, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
        let mut sound = envelop!(triangle(), pv, 0) + envelop!(triangle(), pv, 1) + envelop!(triangle(), pv, 2) + envelop!(triangle(), pv, 3) + envelop!(triangle(), pv, 4) + envelop!(triangle(), pv, 5) + envelop!(triangle(), pv, 6) + envelop!(triangle(), pv, 7);// + envelop!(triangle(), pv, 8) + envelop!(triangle(), pv, 9) + envelop!(triangle(), pv, 10) + envelop!(triangle(), pv, 11)+ envelop!(triangle(), pv, 12) + envelop!(triangle(), pv, 13) + envelop!(triangle(), pv, 14) + envelop!(triangle(), pv, 15);
        sound.reset(Some(sample_rate));
        let mut next_value = move || sound.get_stereo();
        let channels = config.channels as usize;
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

fn create_sound(incoming_pitch: An<Var<f64>>, incoming_velocity: An<Var<f64>>) -> impl AudioUnit64 {
    envelope(move |_t| midi_hz(incoming_pitch.value())) >> triangle() * envelope(move |_t| (incoming_velocity.value() / 127.0))
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
