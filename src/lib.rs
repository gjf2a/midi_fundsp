use anyhow::bail;
use bare_metal_modulo::*;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Sample, SampleFormat, StreamConfig,
};
use crossbeam_queue::SegQueue;
use fundsp::hacker::{
    midi_hz, shared, triangle, var, An, AudioUnit64, FrameAdd, Net64, Shared, Var,
};
use midi_msg::{ChannelVoiceMsg, MidiMsg};
use midir::{Ignore, MidiInput, MidiInputPort};
use std::{collections::BTreeMap, sync::Arc};

pub fn start_input_thread(
    midi_msgs: Arc<SegQueue<MidiMsg>>,
    midi_in: MidiInput,
    in_port: MidiInputPort,
) {
    std::thread::spawn(move || {
        let _conn_in = midi_in
            .connect(
                &in_port,
                "midir-read-input",
                move |_stamp, message, _| {
                    let (msg, _len) = MidiMsg::from_midi(&message).unwrap();
                    midi_msgs.push(msg);
                },
                (),
            )
            .unwrap();
        loop {}
    });
}

pub fn get_first_midi_device(midi_in: &mut MidiInput) -> anyhow::Result<MidiInputPort> {
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

pub fn velocity2volume(velocity: u8) -> f64 {
    velocity as f64 / 127.0
}

pub trait Player {
    fn sound(&self) -> Net64;
    fn on(&mut self, pitch: u8, velocity: u8);
    fn off(&mut self, pitch: u8);

    fn listen(&mut self, midi_msgs: Arc<SegQueue<MidiMsg>>) {
        loop {
            if let Some(msg) = midi_msgs.pop() {
                self.decode(msg);
            }
        }
    }

    fn decode(&mut self, msg: MidiMsg) {
        match msg {
            MidiMsg::ChannelVoice { channel: _, msg } => match msg {
                ChannelVoiceMsg::NoteOn { note, velocity } => {
                    self.on(note, velocity);
                }
                ChannelVoiceMsg::NoteOff { note, velocity: _ } => {
                    self.off(note);
                }
                _ => {}
            },
            _ => {}
        }
    }
}

pub fn run_output<P: Player + Send + Sync>(player: P, midi_msgs: Arc<SegQueue<MidiMsg>>) {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config().unwrap();
    match config.sample_format() {
        SampleFormat::F32 => run_synth::<P, f32>(player, midi_msgs, device, config.into()),
        SampleFormat::I16 => run_synth::<P, i16>(player, midi_msgs, device, config.into()),
        SampleFormat::U16 => run_synth::<P, u16>(player, midi_msgs, device, config.into()),
    };
}

fn run_synth<P: Player, T: Sample>(
    mut player: P,
    midi_msgs: Arc<SegQueue<MidiMsg>>,
    device: Device,
    config: StreamConfig,
) {
    let sample_rate = config.sample_rate.0 as f64;
    let mut sound = player.sound();
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
    player.listen(midi_msgs);
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

#[derive(Clone)]
pub struct LiveSounds<const N: usize> {
    pitches: [Shared<f64>; N],
    velocities: [Shared<f64>; N],
    controls: [Shared<f64>; N],
    next: ModNumC<usize, N>,
    pitch2var: BTreeMap<u8, usize>,
    recent_pitches: [Option<u8>; N],
    synth_func:
        Arc<dyn Fn(An<Var<f64>>, An<Var<f64>>, An<Var<f64>>) -> Box<dyn AudioUnit64> + Send + Sync>,
}

impl<const N: usize> Player for LiveSounds<N> {
    fn sound(&self) -> Net64 {
        let mut sound = Net64::wrap(self.sound_at(0));
        for i in 1..N {
            sound = Net64::bin_op(sound, Net64::wrap(self.sound_at(i)), FrameAdd::new());
        }
        sound
    }

    fn on(&mut self, pitch: u8, velocity: u8) {
        self.pitches[self.next.a()].set_value(midi_hz(pitch as f64));
        self.velocities[self.next.a()].set_value(velocity2volume(velocity));
        self.controls[self.next.a()].set_value(1.0);
        self.pitch2var.insert(pitch, self.next.a());
        self.recent_pitches[self.next.a()] = Some(pitch);
        self.next += 1;
    }

    fn off(&mut self, pitch: u8) {
        if let Some(i) = self.pitch2var.remove(&pitch) {
            if self.recent_pitches[i] == Some(pitch) {
                self.release(i);
            }
        }
    }
}

impl<const N: usize> LiveSounds<N> {
    pub fn new(
        synth_func: Arc<
            dyn Fn(An<Var<f64>>, An<Var<f64>>, An<Var<f64>>) -> Box<dyn AudioUnit64> + Send + Sync,
        >,
    ) -> Self {
        Self {
            pitches: [(); N].map(|_| shared(0.0)),
            velocities: [(); N].map(|_| shared(0.0)),
            controls: [(); N].map(|_| shared(0.0)),
            next: ModNumC::new(0),
            pitch2var: BTreeMap::new(),
            recent_pitches: [None; N],
            synth_func,
        }
    }

    pub fn sound_at(&self, i: usize) -> Box<dyn AudioUnit64> {
        (self.synth_func)(
            var(&self.pitches[i]),
            var(&self.velocities[i]),
            var(&self.controls[i]),
        )
    }

    fn release(&mut self, i: usize) {
        self.recent_pitches[i] = None;
        self.controls[i].set_value(0.0);
    }

    pub fn all_off(&mut self) {
        for i in 0..N {
            self.release(i);
        }
    }
}

pub fn simple_triangle(
    pitch: An<Var<f64>>,
    velocity: An<Var<f64>>,
    control: An<Var<f64>>,
) -> Box<dyn AudioUnit64> {
    Box::new(pitch >> triangle() * velocity * control)
}
