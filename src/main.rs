use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Sample, SampleFormat, StreamConfig};
use fundsp::hacker::*;
use fundsp::prelude::AudioUnit64;

use bare_metal_modulo::*;

use std::collections::BTreeMap;
use std::{thread, time};

const NUM_TO_USE: usize = 10;

const NOTES: [(u8, u8); 10] = [
    (60, 127),
    (62, 100),
    (64, 127),
    (65, 50),
    (67, 80),
    (69, 100),
    (71, 60),
    (72, 127),
    (74, 100),
    (76, 127),
];

fn main() -> anyhow::Result<()> {
    let mut vars: Vars<NUM_TO_USE> = Vars::new();
    run_output(vars.clone());

    let rest = time::Duration::from_secs(1);
    thread::sleep(rest);
    for i in 0..NOTES.len() {
        println!("{i}");
        vars.on(NOTES[i].0, NOTES[i].1);
        thread::sleep(rest);
    }
    Ok(())
}

fn run_output<const N: usize>(vars: Vars<N>) {
    thread::spawn(move || {
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

#[derive(Clone)]
struct Vars<const N: usize> {
    pitches: [An<Var<f64>>; N],
    velocities: [An<Var<f64>>; N],
    next: ModNumC<usize, N>,
    pitch2var: BTreeMap<u8, usize>,
    recent_pitches: [Option<u8>; N],
}

impl<const N: usize> Vars<N> {
    pub fn new() -> Self {
        Self {
            pitches: [(); N].map(|_| var(0, 0.0)),
            velocities: [(); N].map(|_| var(1, 0.0)),
            next: ModNumC::new(0),
            pitch2var: BTreeMap::new(),
            recent_pitches: [None; N],
        }
    }

    pub fn sound_at(&self, i: usize) -> Box<dyn AudioUnit64> {
        let pitch = self.pitches[i].clone();
        let velocity = self.velocities[i].clone();
        Box::new(
            envelope(move |_| midi_hz(pitch.value()))
                >> triangle() * (envelope(move |_| velocity.value() / 127.0)),
        )
    }

    pub fn sound(&self) -> Net64 {
        let mut sound = Net64::wrap(self.sound_at(0));
        for i in 1..N {
            sound = Net64::bin_op(sound, Net64::wrap(self.sound_at(i)), FrameAdd::new());
        }
        sound
    }

    pub fn on(&mut self, pitch: u8, velocity: u8) {
        self.pitches[self.next.a()].clone().set_value(pitch as f64);
        self.velocities[self.next.a()]
            .clone()
            .set_value(velocity as f64);
        self.pitch2var.insert(pitch, self.next.a());
        self.recent_pitches[self.next.a()] = Some(pitch);
        self.next += 1;
    }
}

fn run_synth<const N: usize, T: Sample>(vars: Vars<N>, device: Device, config: StreamConfig) {
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
