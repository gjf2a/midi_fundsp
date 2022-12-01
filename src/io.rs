use anyhow::{anyhow, bail};
use bare_metal_modulo::*;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Sample, SampleFormat, Stream, StreamConfig,
};
use crossbeam_queue::SegQueue;
use fundsp::hacker::{shared, var, AudioUnit64, FrameAdd, FrameMul, Net64, Shared};
use midi_msg::{Channel, ChannelVoiceMsg, MidiMsg};
use midir::{Ignore, MidiInput, MidiInputPort};
use std::sync::Arc;

use crate::{SharedMidiState, SynthFunc, MAX_MIDI_VALUE};

const NUM_MIDI_VALUES: usize = MAX_MIDI_VALUE as usize + 1;

#[derive(Clone)]
pub enum SynthMsg {
    Midi(MidiMsg, Speaker),
    SetSynth(SynthFunc, Speaker),
    Off(Speaker),
    Quit,
}

impl SynthMsg {
    pub fn speaker_swapped(&self, new_speaker: Speaker) -> Self {
        match self {
            SynthMsg::Midi(m, _) => SynthMsg::Midi(m.clone(), new_speaker),
            SynthMsg::SetSynth(s, _) => SynthMsg::SetSynth(s.clone(), new_speaker),
            SynthMsg::Off(_) => SynthMsg::Off(new_speaker),
            SynthMsg::Quit => SynthMsg::Quit,
        }
    }

    pub fn speaker(&self) -> Option<Speaker> {
        match self {
            SynthMsg::Midi(_, s) | SynthMsg::SetSynth(_, s) | SynthMsg::Off(s) => Some(*s),
            SynthMsg::Quit => None,
        }
    }
}

pub fn start_input_thread(
    midi_msgs: Arc<SegQueue<SynthMsg>>,
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
                    midi_msgs.push(SynthMsg::Midi(msg, Speaker::Both));
                },
                (),
            )
            .unwrap();
        loop {}
    });
}

#[derive(Copy, Clone)]
pub enum Speaker {
    Left,
    Right,
    Both,
}

impl Speaker {
    pub fn i(&self) -> usize {
        *self as usize
    }
}

pub struct StereoPlayer<const N: usize> {
    sounds: [MonoPlayer<N>; 2],
}

impl<const N: usize> StereoPlayer<N> {
    pub fn mono(synth: SynthFunc) -> Self {
        let sounds = [
            MonoPlayer::<N>::new(synth.clone()),
            MonoPlayer::<N>::new(synth.clone()),
        ];
        Self { sounds }
    }

    pub fn stereo(left_synth: SynthFunc, right_synth: SynthFunc) -> Self {
        let sounds = [
            MonoPlayer::<N>::new(left_synth),
            MonoPlayer::<N>::new(right_synth),
        ];
        Self { sounds }
    }

    fn sound(&self) -> Net64 {
        Net64::stack_op(
            self.sounds[Speaker::Left.i()].sound(),
            self.sounds[Speaker::Right.i()].sound(),
        )
    }

    pub fn run_output(&mut self, midi_msgs: Arc<SegQueue<SynthMsg>>) -> anyhow::Result<()> {
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

    fn act<F: FnMut(&mut MonoPlayer<N>)>(&mut self, speaker: Speaker, mut action: F) {
        match speaker {
            Speaker::Left | Speaker::Right => action(&mut self.sounds[speaker.i()]),
            Speaker::Both => {
                for sound in self.sounds.iter_mut() {
                    action(sound);
                }
            }
        }
    }

    fn run_synth<T: Sample>(
        &mut self,
        midi_msgs: Arc<SegQueue<SynthMsg>>,
        device: Device,
        config: StreamConfig,
    ) -> anyhow::Result<()> {
        Self::warm_up(midi_msgs.clone());
        let mut running = true;
        while running {
            let stream = self.get_stream::<T>(&config, &device)?;
            stream.play()?;
            self.handle_messages(&mut running, midi_msgs.clone());
        }
        Ok(())
    }

    fn warm_up(midi_msgs: Arc<SegQueue<SynthMsg>>) {
        for _ in 0..N {
            midi_msgs.push(Self::warm_up_msg(ChannelVoiceMsg::NoteOn {
                note: 0,
                velocity: 0,
            }));
            midi_msgs.push(Self::warm_up_msg(ChannelVoiceMsg::NoteOff {
                note: 0,
                velocity: 0,
            }));
        }
    }

    fn warm_up_msg(msg: ChannelVoiceMsg) -> SynthMsg {
        SynthMsg::Midi(
            MidiMsg::ChannelVoice {
                channel: Channel::Ch1,
                msg,
            },
            Speaker::Both,
        )
    }

    fn handle_messages(&mut self, running: &mut bool, midi_msgs: Arc<SegQueue<SynthMsg>>) {
        let mut synth_changed = false;
        while !synth_changed {
            if let Some(msg) = midi_msgs.pop() {
                match msg {
                    SynthMsg::Midi(midi_msg, speaker) => self.act(speaker, |s| s.decode(&midi_msg)),
                    SynthMsg::SetSynth(synth, speaker) => {
                        self.act(speaker, |s| s.change_synth(synth.clone()));
                        synth_changed = true;
                    }
                    SynthMsg::Off(speaker) => self.act(speaker, |s| s.release_all()),
                    SynthMsg::Quit => {
                        *running = false;
                    }
                }
            }
        }
    }

    fn get_stream<T: Sample>(
        &self,
        config: &StreamConfig,
        device: &Device,
    ) -> anyhow::Result<Stream> {
        let sample_rate = config.sample_rate.0 as f64;
        let mut sound = self.sound();
        sound.reset(Some(sample_rate));
        let mut next_value = move || sound.get_stereo();
        let channels = config.channels as usize;
        let err_fn = |err| eprintln!("Error on stream: {err}");
        device
            .build_output_stream(
                &config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    write_data(data, channels, &mut next_value)
                },
                err_fn,
            )
            .or_else(|err| bail!("{err:?}"))
    }
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
struct MonoPlayer<const N: usize> {
    states: [SharedMidiState; N],
    next: ModNumC<usize, N>,
    pitch2state: [Option<usize>; NUM_MIDI_VALUES],
    recent_pitches: [Option<u8>; N],
    synth_func: SynthFunc,
    master_volume: Shared<f64>,
}

impl<const N: usize> MonoPlayer<N> {
    fn new(synth_func: SynthFunc) -> Self {
        Self {
            states: [(); N].map(|_| SharedMidiState::default()),
            next: ModNumC::new(0),
            pitch2state: [None; NUM_MIDI_VALUES],
            recent_pitches: [None; N],
            synth_func,
            master_volume: shared(1.0),
        }
    }

    fn sound(&self) -> Net64 {
        let mut sound = Net64::wrap(self.sound_at(0));
        for i in 1..N {
            sound = Net64::bin_op(sound, Net64::wrap(self.sound_at(i)), FrameAdd::new());
        }
        Net64::bin_op(
            sound,
            Net64::wrap(Box::new(var(&self.master_volume))),
            FrameMul::new(),
        )
    }

    fn decode(&mut self, msg: &MidiMsg) {
        match msg {
            MidiMsg::ChannelVoice { channel: _, msg } => match msg {
                ChannelVoiceMsg::NoteOn { note, velocity } => {
                    self.on(*note, *velocity);
                }
                ChannelVoiceMsg::NoteOff { note, velocity: _ } => {
                    self.off(*note);
                }
                ChannelVoiceMsg::PitchBend { bend } => {
                    self.bend(*bend);
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn on(&mut self, pitch: u8, velocity: u8) {
        self.master_volume.set_value(1.0);
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

    fn change_synth(&mut self, new_synth: SynthFunc) {
        self.master_volume.set_value(0.0);
        self.synth_func = new_synth;
    }

    fn bend(&mut self, bend: u16) {
        for state in self.states.iter_mut() {
            state.bend(bend);
        }
    }

    fn sound_at(&self, i: usize) -> Box<dyn AudioUnit64> {
        (self.synth_func)(&self.states[i])
    }

    fn release(&mut self, i: usize) {
        self.recent_pitches[i] = None;
        self.states[i].off();
    }

    fn release_all(&mut self) {
        for i in 0..N {
            self.release(i);
        }
    }
}