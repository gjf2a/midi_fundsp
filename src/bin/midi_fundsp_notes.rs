use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Sample, SampleFormat, StreamConfig};
use fundsp::hacker::*;
use fundsp::prelude::{AudioUnit64};
use midi_msg::{ChannelVoiceMsg, MidiMsg};
use midir::{Ignore, MidiInput, MidiInputPort};
use read_input::prelude::*;
use bare_metal_modulo::*;
use anyhow::bail;
use std::collections::BTreeMap;

fn main() -> anyhow::Result<()> {
    let mut midi_in = MidiInput::new("midir reading input")?;
    let in_port = get_midi_device(&mut midi_in)?;

    let vars: Vars<6> = Vars::new();
    run_output(vars.clone());
    run_input(vars, midi_in, in_port)
}

fn run_input<const N: usize>(
    mut vars: Vars<N>,
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
                                println!("on: {note} {velocity}");
                                vars.on(note, velocity);
                            }
                            ChannelVoiceMsg::NoteOff { note, velocity:_ } => {
                                println!("off: {note}");
                                vars.off(note);
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

fn run_output<const N: usize>(vars: Vars<N>) {
    std::thread::spawn(move || {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("failed to find a default output device");
        let config = device.default_output_config().unwrap();
        match config.sample_format() {
            SampleFormat::F32 => run_synth::<N, f32>(vars, device, config.into()),
            SampleFormat::I16 => run_synth::<N, i16>(vars, device, config.into()),
            SampleFormat::U16 => run_synth::<N, u16>(vars, device, config.into()),
        };
    });
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

#[derive(Clone)]
struct Vars<const N: usize> {
    pitches: [Shared<f64>; N],
    velocities: [Shared<f64>; N],
    next: ModNumC<usize, N>,
    pitch2var: BTreeMap<u8,usize>,
    recent_pitches: [Option<u8>; N]
}

impl <const N: usize> Vars<N> {
    pub fn new() -> Self {
        Self {
            pitches: [(); N].map(|_| shared(0.0)),
            velocities: [(); N].map(|_| shared(0.0)),
            next: ModNumC::new(0),
            pitch2var: BTreeMap::new(),
            recent_pitches: [None; N]
        }
    }

    pub fn sound_at(&self, i: usize) -> Box<dyn AudioUnit64> {
        Box::new(var(&self.pitches[i]) >> triangle() * var(&self.velocities[i]))
    }

    pub fn sound(&self) -> Net64 {
        let mut sound = Net64::wrap(self.sound_at(0));
        for i in 1..N {
            sound = Net64::bin_op(sound, Net64::wrap(self.sound_at(i)), FrameAdd::new());
        }
        sound
    }

    pub fn on(&mut self, pitch: u8, velocity: u8) {
        self.pitches[self.next.a()].set_value(midi_hz(pitch as f64));
        self.velocities[self.next.a()].set_value(velocity as f64 / 127.0);
        self.pitch2var.insert(pitch, self.next.a());
        self.recent_pitches[self.next.a()] = Some(pitch);
        self.next += 1;
    }

    pub fn off(&mut self, pitch: u8) {
        if let Some(i) = self.pitch2var.remove(&pitch) {
            if self.recent_pitches[i] == Some(pitch) {
                self.recent_pitches[i] = None;
                self.velocities[i].clone().set_value(0.0);
            }
        }
    }
}

fn run_synth<const N: usize, T: Sample>(
    vars: Vars<N>,
    device: Device,
    config: StreamConfig,
) {
    let sample_rate = config.sample_rate.0 as f64;
    let mut sound = vars.sound();
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