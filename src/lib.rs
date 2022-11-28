use anyhow::{anyhow, bail};
use bare_metal_modulo::*;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Sample, SampleFormat, StreamConfig,
};
use crossbeam_queue::SegQueue;
use fundsp::hacker::{midi_hz, clamp01, triangle, var, An, AudioUnit64, FrameAdd, Net64, Shared, Var, envelope};
use midi_msg::{ChannelVoiceMsg, MidiMsg};
use midir::{Ignore, MidiInput, MidiInputPort};
use std::sync::Arc;

pub const MAX_MIDI_VALUE: u8 = 127;
const NUM_MIDI_VALUES: usize = MAX_MIDI_VALUE as usize + 1;

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
        let device_name = midi_in.port_name(&in_ports[0])?;
        println!("Chose MIDI device {device_name}");
        Ok(in_ports[0].clone())
    }
}

pub trait Player: Send + Sync {
    type Msg;

    fn sound(&self) -> Net64;

    fn decode(&mut self, msg: Self::Msg);

    fn listen(&mut self, midi_msgs: Arc<SegQueue<Self::Msg>>);

    fn run_output(&mut self, midi_msgs: Arc<SegQueue<Self::Msg>>) -> anyhow::Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(anyhow!("failed to find a default output device"))?;
        let config = device.default_output_config()?;
        match config.sample_format() {
            SampleFormat::F32 => self.run_synth::<f32>(midi_msgs, device, config.into()),
            SampleFormat::I16 => self.run_synth::<i16>(midi_msgs, device, config.into()),
            SampleFormat::U16 => self.run_synth::<u16>(midi_msgs, device, config.into()),
        }
    }

    fn run_synth<T: Sample>(
        &mut self,
        midi_msgs: Arc<SegQueue<Self::Msg>>,
        device: Device,
        config: StreamConfig,
    ) -> anyhow::Result<()> {
        let sample_rate = config.sample_rate.0 as f64;
        let mut sound = self.sound();
        sound.reset(Some(sample_rate));
        let mut next_value = move || sound.get_stereo();
        let channels = config.channels as usize;
        let err_fn = |err| eprintln!("an error occurred on stream: {err}");
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                write_data(data, channels, &mut next_value)
            },
            err_fn,
        )?;

        stream.play()?;
        self.listen(midi_msgs);
        Ok(())
    }
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

#[derive(Clone, Default)]
pub struct SharedMidiState {
    pitch: Shared<f64>,
    velocity: Shared<f64>,
    control: Shared<f64>,
}

impl SharedMidiState {
    pub fn pitch_velocity_control_vars(&self) -> (An<Var<f64>>, An<Var<f64>>, An<Var<f64>>) {
        (var(&self.pitch), var(&self.velocity), var(&self.control))
    }

    pub fn on(&mut self, pitch: u8, velocity: u8) {
        self.pitch.set_value(midi_hz(pitch as f64));
        self.velocity
            .set_value(velocity as f64 / MAX_MIDI_VALUE as f64);
        self.control.set_value(1.0);
    }

    pub fn off(&mut self) {
        self.control.set_value(-1.0);
    }
}

pub type SynthFunc = dyn Fn(&SharedMidiState) -> Box<dyn AudioUnit64> + Send + Sync;

#[derive(Clone)]
pub struct LiveSounds<const N: usize> {
    states: [SharedMidiState; N],
    next: ModNumC<usize, N>,
    pitch2state: [Option<usize>; NUM_MIDI_VALUES],
    recent_pitches: [Option<u8>; N],
    synth_func: Arc<SynthFunc>,
}

impl<const N: usize> Player for LiveSounds<N> {
    type Msg = MidiMsg;

    fn sound(&self) -> Net64 {
        let mut sound = Net64::wrap(self.sound_at(0));
        for i in 1..N {
            sound = Net64::bin_op(sound, Net64::wrap(self.sound_at(i)), FrameAdd::new());
        }
        sound
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

    fn listen(&mut self, midi_msgs: Arc<SegQueue<MidiMsg>>) {
        loop {
            if let Some(msg) = midi_msgs.pop() {
                self.decode(msg);
            }
        }
    }
}

impl<const N: usize> LiveSounds<N> {
    pub fn new(synth_func: Arc<SynthFunc>) -> Self {
        Self {
            states: [(); N].map(|_| SharedMidiState::default()),
            next: ModNumC::new(0),
            pitch2state: [None; NUM_MIDI_VALUES],
            recent_pitches: [None; N],
            synth_func,
        }
    }

    fn on(&mut self, pitch: u8, velocity: u8) {
        self.states[self.next.a()].on(pitch, velocity);
        self.pitch2state[pitch as usize] = Some(self.next.a());
        self.recent_pitches[self.next.a()] = Some(pitch);
        self.next += 1;
    }

    fn off(&mut self, pitch: u8) {
        if let Some(i) = self.pitch2state[pitch as usize] {
            if self.recent_pitches[i] == Some(pitch) {
                self.release(i);
            }
            self.pitch2state[pitch as usize] = None;
        }
    }

    pub fn sound_at(&self, i: usize) -> Box<dyn AudioUnit64> {
        (self.synth_func)(&self.states[i])
    }

    fn release(&mut self, i: usize) {
        self.recent_pitches[i] = None;
        self.states[i].off();
    }

    pub fn all_off(&mut self) {
        for i in 0..N {
            self.release(i);
        }
    }
}

pub struct StereoSounds<const N: usize> {
    sounds: [LiveSounds<N>; 2],
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum StereoSide {
    Left,
    Right,
}

pub struct StereoMsg {
    pub midi_msg: MidiMsg,
    pub side: StereoSide,
}

impl<const N: usize> Player for StereoSounds<N> {
    type Msg = StereoMsg;

    fn sound(&self) -> Net64 {
        Net64::stack_op(
            self.sounds[StereoSide::Left.i()].sound(),
            self.sounds[StereoSide::Right.i()].sound(),
        )
    }

    fn decode(&mut self, msg: Self::Msg) {
        self.sounds[msg.side.i()].decode(msg.midi_msg);
    }

    fn listen(&mut self, midi_msgs: Arc<SegQueue<Self::Msg>>) {
        loop {
            if let Some(msg) = midi_msgs.pop() {
                self.decode(msg);
            }
        }
    }
}

impl StereoSide {
    pub fn i(&self) -> usize {
        *self as usize
    }
}

impl<const N: usize> StereoSounds<N> {
    pub fn new(left_synth_func: Arc<SynthFunc>, right_synth_func: Arc<SynthFunc>) -> Self {
        Self {
            sounds: [
                LiveSounds::new(left_synth_func),
                LiveSounds::new(right_synth_func),
            ],
        }
    }

    fn side(&mut self, side: StereoSide) -> &mut LiveSounds<N> {
        &mut self.sounds[side.i()]
    }

    pub fn note_on(&mut self, pitch: u8, velocity: u8, side: StereoSide) {
        self.side(side).on(pitch, velocity)
    }

    pub fn note_off(&mut self, pitch: u8, side: StereoSide) {
        self.side(side).off(pitch)
    }

    pub fn all_off(&mut self) {
        self.sounds.iter_mut().for_each(|s| s.all_off());
    }
}

pub fn simple_triangle(shared_midi_state: &SharedMidiState) -> Box<dyn AudioUnit64> {
    let (pitch, velocity, control) = shared_midi_state.pitch_velocity_control_vars();
    Box::new(pitch >> triangle() * velocity * envelope(move |_| clamp01(control.value())))
}
