use anyhow::{anyhow, bail};
use bare_metal_modulo::*;
use cpal::{
    Device, FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossbeam_queue::SegQueue;
use crossbeam_utils::atomic::AtomicCell;
use fundsp::{
    net::Net,
    prelude::{AudioUnit, FrameAdd, FrameMul},
    prelude64::{shared, var},
    shared::Shared,
};
use midi_msg::{Channel, ChannelModeMsg, ChannelVoiceMsg, MidiMsg, SystemRealTimeMsg};
use midir::{Ignore, MidiInput, MidiInputPort};
use read_input::{InputBuild, shortcut::input};
use std::sync::{Arc, Mutex};

use crate::{
    NUM_MIDI_VALUES, SharedMidiState, SynthFunc, note_velocity_from, sound_builders::ProgramTable,
};

#[derive(Clone, Debug)]
/// Packages a [`MidiMsg`](https://crates.io/crates/midi-msg) with a designated `Speaker` to output the sound
/// corresponding to the message.
pub struct SynthMsg {
    pub msg: MidiMsg,
    pub speaker: Speaker,
}

impl SynthMsg {
    /// Returns MIDI `All Notes Off` message. This releases all current sounds.
    pub fn all_notes_off(speaker: Speaker) -> Self {
        Self::mode_msg(ChannelModeMsg::AllNotesOff, speaker)
    }

    /// Returns MIDI `All Sound Off` message. This shuts off all current sounds immediately.
    pub fn all_sound_off(speaker: Speaker) -> Self {
        Self::mode_msg(ChannelModeMsg::AllSoundOff, speaker)
    }

    fn mode_msg(msg: ChannelModeMsg, speaker: Speaker) -> Self {
        Self {
            msg: MidiMsg::ChannelMode {
                channel: midi_msg::Channel::Ch1,
                msg,
            },
            speaker,
        }
    }

    /// Returns MIDI `System Reset` message.
    pub fn system_reset(speaker: Speaker) -> Self {
        Self::system_real_time_msg(SystemRealTimeMsg::SystemReset, speaker)
    }

    fn system_real_time_msg(msg: SystemRealTimeMsg, speaker: Speaker) -> Self {
        Self {
            msg: MidiMsg::SystemRealTime { msg },
            speaker,
        }
    }

    /// Returns MIDI `Program Change` message. This selects the synthesizer sound with the given index.
    pub fn program_change(program: u8, speaker: Speaker) -> Self {
        Self {
            msg: MidiMsg::ChannelVoice {
                channel: midi_msg::Channel::Ch1,
                msg: ChannelVoiceMsg::ProgramChange { program },
            },
            speaker,
        }
    }

    /// Returns MIDI note and velocity information if pertinent
    pub fn note_velocity(&self) -> Option<(u8, u8)> {
        note_velocity_from(&self.msg)
    }
}

/// Starts a thread that monitors MIDI input events from the source specified by `in_port`. Each message received is
/// stored in a `SynthMsg` object and placed in the `midi_msgs` queue.
///
/// If `true` is stored in `quit`, the thread exits and it sends a MIDI `SystemReset` message.
/// If `print_incoming_msg` is `true`, each incoming MIDI message will be printed to the console.
///
/// The functions `get_first_midi_device()` and `choose_midi_device()` are examples of how to
/// select a value for `in_port`.
pub fn start_input_thread(
    midi_msgs: Arc<SegQueue<SynthMsg>>,
    midi_in: MidiInput,
    in_port: MidiInputPort,
    quit: Arc<AtomicCell<bool>>,
) {
    start_generic_input_thread(
        |msg| SynthMsg {
            msg,
            speaker: Speaker::Both,
        },
        SynthMsg::system_reset(Speaker::Both),
        midi_msgs,
        midi_in,
        in_port,
        quit,
    )
}

/// Starts a thread that monitors MIDI input events from the source specified by `in_port`. Each `MidiMsg` object
/// received is placed in the `midi_msgs` queue.
///
/// If `true` is stored in `quit`, the thread exits and it sends a MIDI `SystemReset` message.
/// If `print_incoming_msg` is `true`, each incoming MIDI message will be printed to the console.
///
/// The functions `get_first_midi_device()` and `choose_midi_device()` are examples of how to
/// select a value for `in_port`.
pub fn start_midi_input_thread(
    midi_msgs: Arc<SegQueue<MidiMsg>>,
    midi_in: MidiInput,
    in_port: MidiInputPort,
    quit: Arc<AtomicCell<bool>>,
) {
    start_generic_input_thread(
        |msg| msg,
        MidiMsg::SystemRealTime {
            msg: SystemRealTimeMsg::SystemReset,
        },
        midi_msgs,
        midi_in,
        in_port,
        quit,
    )
}

fn start_generic_input_thread<M: Send + 'static, F: Send + 'static + Fn(MidiMsg) -> M>(
    encoder: F,
    reset: M,
    midi_msgs: Arc<SegQueue<M>>,
    midi_in: MidiInput,
    in_port: MidiInputPort,
    quit: Arc<AtomicCell<bool>>,
) {
    std::thread::spawn(move || {
        let _conn_in = midi_in
            .connect(
                &in_port,
                "midir-read-input",
                input_callback(encoder, midi_msgs.clone()),
                (),
            )
            .unwrap();
        while !quit.load() {}
        midi_msgs.push(reset);
        quit.store(false);
    });
}

fn input_callback<M: Send + 'static, F: Send + 'static + Fn(MidiMsg) -> M>(
    encoder: F,
    midi_msgs: Arc<SegQueue<M>>,
) -> impl Fn(u64, &[u8], &mut ()) {
    move |_stamp, message, _| {
        let (msg, _len) = MidiMsg::from_midi(&message).unwrap();
        midi_msgs.push(encoder(msg));
    }
}

/// Plays sounds according to instructions received in the `midi_msgs` queue. Synthesizer sounds may be selected with
/// MIDI `Program Change` messages that reference sounds stored in `program_table`.
///
/// The constant value `N` is the number of distinct sounds it can emit. Each MIDI `Note On` message uses one distinct
/// sound. When a number of `Note On` messages greater than `N` has been received, the sound used by the oldest `Note On`
/// message is reused for the new `Note On` message.
///
/// Setting `N = 1` yields a monophonic synthesizer. Setting `N = 10` should suffice for most purposes.
///
/// If a `SystemReset` MIDI message is received, the thread exits.
pub fn start_output_thread<const N: usize>(
    midi_msgs: Arc<SegQueue<SynthMsg>>,
    program_table: Arc<Mutex<ProgramTable>>,
) {
    std::thread::spawn(move || {
        let mut player = StereoPlayer::<N>::new(program_table);
        player.run_output(midi_msgs).unwrap();
    });
}

/// Plays sounds according to `MidiMsg` objects received in the `midi_msgs` queue. Synthesizer sounds may be selected with
/// MIDI `Program Change` messages that reference sounds stored in `program_table`.
///
/// The constant value `N` is the number of distinct sounds it can emit. Each MIDI `Note On` message uses one distinct
/// sound. When a number of `Note On` messages greater than `N` has been received, the sound used by the oldest `Note On`
/// message is reused for the new `Note On` message.
///
/// Setting `N = 1` yields a monophonic synthesizer. Setting `N = 10` should suffice for most purposes.
///
/// If a `SystemReset` MIDI message is received, the thread exits.
pub fn start_midi_output_thread<const N: usize>(
    midi_msgs: Arc<SegQueue<MidiMsg>>,
    program_table: Arc<Mutex<ProgramTable>>,
) {
    inner_start_output_thread(midi_msgs, StereoPlayer::<N>::new(program_table));
}

/// Plays sounds according to `MidiMsg` objects received in the `midi_msgs` queue. Synthesizer sounds may be selected with
/// MIDI `Program Change` messages that reference sounds stored in `program_table`.
///
/// The function `midi_to_hz()` converts MIDI pitches (0-127) to frequencies. To represent
/// an alternative tuning system, pass in an appropriate function.
///
/// The constant value `N` is the number of distinct sounds it can emit. Each MIDI `Note On` message uses one distinct
/// sound. When a number of `Note On` messages greater than `N` has been received, the sound used by the oldest `Note On`
/// message is reused for the new `Note On` message.
///
/// Setting `N = 1` yields a monophonic synthesizer. Setting `N = 10` should suffice for most purposes.
///
/// If a `SystemReset` MIDI message is received, the thread exits.
pub fn start_midi_output_thread_alt_tuning<const N: usize>(
    midi_msgs: Arc<SegQueue<MidiMsg>>,
    program_table: Arc<Mutex<ProgramTable>>,
    midi_to_hz: fn(f32) -> f32,
) {
    let mut player = StereoPlayer::<N>::new(program_table);
    player.set_midi_to_hz(midi_to_hz);
    inner_start_output_thread(midi_msgs, player);
}

fn inner_start_output_thread<const N: usize>(
    midi_msgs: Arc<SegQueue<MidiMsg>>,
    mut player: StereoPlayer<N>,
) {
    let relay_out = Arc::new(SegQueue::new());
    let relay_in = relay_out.clone();
    std::thread::spawn(move || {
        loop {
            if let Some(msg) = midi_msgs.pop() {
                relay_out.push(SynthMsg {
                    msg,
                    speaker: Speaker::Both,
                })
            }
        }
    });

    std::thread::spawn(move || {
        player.run_output(relay_in).unwrap();
    });
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
/// Represents whether a sound should go to the left, right, or both speakers.
pub enum Speaker {
    Left,
    Right,
    Both,
}

impl Speaker {
    /// Value for using a `Speaker` as an array index.
    pub fn i(&self) -> usize {
        *self as usize
    }
}

struct StereoPlayer<const N: usize> {
    sounds: [MonoPlayer<N>; 2],
}

impl<const N: usize> StereoPlayer<N> {
    fn new(program_table: Arc<Mutex<ProgramTable>>) -> Self {
        let sounds = [
            MonoPlayer::<N>::new(program_table.clone()),
            MonoPlayer::<N>::new(program_table),
        ];
        Self { sounds }
    }

    fn set_midi_to_hz(&mut self, midi_to_hz: fn(f32) -> f32) {
        for i in 0..self.sounds.len() {
            self.sounds[i].set_midi_to_hz(midi_to_hz);
        }
    }

    fn sound(&self) -> Net {
        Net::stack(
            self.sounds[Speaker::Left.i()].sound(),
            self.sounds[Speaker::Right.i()].sound(),
        )
    }

    fn run_output(&mut self, midi_msgs: Arc<SegQueue<SynthMsg>>) -> anyhow::Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(anyhow!("failed to find a default output device"))?;
        let config = device.default_output_config()?;
        match config.sample_format() {
            SampleFormat::F32 => self.run_synth::<f32>(midi_msgs, device, config.into()),
            SampleFormat::I16 => self.run_synth::<i16>(midi_msgs, device, config.into()),
            SampleFormat::U16 => self.run_synth::<u16>(midi_msgs, device, config.into()),
            sample_format => panic!("Unsupported sample format '{sample_format}'"),
        }
    }

    fn decode(&mut self, speaker: Speaker, msg: &MidiMsg) -> Option<RelayedMessage> {
        match speaker {
            Speaker::Left | Speaker::Right => self.sounds[speaker.i()].decode(msg),
            Speaker::Both => {
                let mut result = None;
                for sound in self.sounds.iter_mut() {
                    result = result.or(sound.decode(msg));
                }
                result
            }
        }
    }

    fn run_synth<T: Sample + SizedSample + FromSample<f32>>(
        &mut self,
        midi_msgs: Arc<SegQueue<SynthMsg>>,
        device: Device,
        config: StreamConfig,
    ) -> anyhow::Result<()> {
        Self::warm_up(midi_msgs.clone());
        let mut done = false;
        while !done {
            let stream = self.get_stream::<T>(&config, &device)?;
            stream.play()?;
            if self.handle_messages(midi_msgs.clone()) == RelayedMessage::SystemReset {
                done = true;
            }
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
        SynthMsg {
            msg: MidiMsg::ChannelVoice {
                channel: Channel::Ch1,
                msg,
            },
            speaker: Speaker::Both,
        }
    }

    fn handle_messages(&mut self, midi_msgs: Arc<SegQueue<SynthMsg>>) -> RelayedMessage {
        loop {
            if let Some(msg) = midi_msgs.pop() {
                if let Some(relayed) = self.decode(msg.speaker, &msg.msg) {
                    return relayed;
                }
            }
        }
    }

    fn get_stream<T: Sample + SizedSample + FromSample<f32>>(
        &self,
        config: &StreamConfig,
        device: &Device,
    ) -> anyhow::Result<Stream> {
        let sample_rate = config.sample_rate as f64;
        let mut sound = self.sound();
        sound.reset();
        sound.set_sample_rate(sample_rate);
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
                None,
            )
            .or_else(|err| bail!("{err:?}"))
    }
}

/// Presents a list of items to be selected via console input. Used in multiple
/// [example](https://github.com/gjf2a/midi_fundsp/tree/master/examples) programs.
pub fn console_choice_from<T, F: Fn(&T) -> &str>(
    prompt: &str,
    choices: &Vec<T>,
    prompt_func: F,
) -> usize {
    for i in 0..choices.len() {
        println!("{}: {}", i + 1, prompt_func(&choices[i]));
    }
    let prompt = format!("{prompt}: ");
    input().msg(prompt).inside(1..=choices.len()).get() - 1
}

/// Returns a handle to the first MIDI device detected.
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

/// Allows selecting a MIDI device via the console from a complete list of MIDI devices.
/// The basic concept can be a model of how to do this in a GUI setting.
pub fn choose_midi_device(midi_in: &mut MidiInput) -> anyhow::Result<MidiInputPort> {
    midi_in.ignore(Ignore::None);
    let in_ports = midi_in.ports();
    match in_ports.len() {
        0 => bail!("No MIDI devices attached"),
        1 => get_first_midi_device(midi_in),
        _ => {
            let mut choices = vec![];
            for port in in_ports.iter() {
                choices.push((midi_in.port_name(port)?, port));
            }
            let c = console_choice_from("Select MIDI Device", &choices, |choice| choice.0.as_str());
            Ok(choices[c].1.clone())
        }
    }
}

fn write_data<T: Sample + FromSample<f32>>(
    output: &mut [T],
    channels: usize,
    next_sample: &mut dyn FnMut() -> (f32, f32),
) {
    for frame in output.chunks_mut(channels) {
        let sample = next_sample();
        let left: T = Sample::from_sample::<f32>(sample.0);
        let right: T = Sample::from_sample::<f32>(sample.1);

        for (channel, sample) in frame.iter_mut().enumerate() {
            *sample = if channel & 1 == 0 { left } else { right };
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum RelayedMessage {
    SynthChange,
    SystemReset,
}

#[derive(Clone)]
struct MonoPlayer<const N: usize> {
    states: [SharedMidiState; N],
    next: ModNumC<usize, N>,
    pitch2state: [Option<usize>; NUM_MIDI_VALUES],
    recent_pitches: [Option<u8>; N],
    synth_func: SynthFunc,
    master_volume: Shared,
    program_table: Arc<Mutex<ProgramTable>>,
}

impl<const N: usize> MonoPlayer<N> {
    fn new(program_table: Arc<Mutex<ProgramTable>>) -> Self {
        let synth_func = {
            let program_table = program_table.lock().unwrap();
            program_table[0].1.clone()
        };
        Self {
            states: [(); N].map(|_| SharedMidiState::default()),
            next: ModNumC::new(0),
            pitch2state: [None; NUM_MIDI_VALUES],
            recent_pitches: [None; N],
            synth_func,
            master_volume: shared(1.0),
            program_table,
        }
    }

    fn set_midi_to_hz(&mut self, midi_to_hz: fn(f32) -> f32) {
        for i in 0..self.states.len() {
            self.states[i].set_midi_to_hz(midi_to_hz);
        }
    }

    fn sound(&self) -> Net {
        let mut sound = Net::wrap(self.sound_at(0));
        for i in 1..N {
            sound = Net::binary(sound, Net::wrap(self.sound_at(i)), FrameAdd::new());
        }
        Net::binary(
            sound,
            Net::wrap(Box::new(var(&self.master_volume))),
            FrameMul::new(),
        )
    }

    fn decode(&mut self, msg: &MidiMsg) -> Option<RelayedMessage> {
        match msg {
            MidiMsg::ChannelVoice { channel: _, msg } => match msg {
                ChannelVoiceMsg::NoteOn { note, velocity } => {
                    if *velocity == 0_u8 {
                        self.off(*note);
                    } else {
                        self.on(*note, *velocity);
                    }
                }
                ChannelVoiceMsg::NoteOff { note, velocity: _ } => {
                    self.off(*note);
                }
                ChannelVoiceMsg::PitchBend { bend } => {
                    self.bend(*bend);
                }
                ChannelVoiceMsg::ProgramChange { program } => {
                    let new_synth = {
                        let program_table = self.program_table.lock().unwrap();
                        program_table[*program as usize].1.clone()
                    };
                    self.change_synth(new_synth);
                    return Some(RelayedMessage::SynthChange);
                }
                _ => {}
            },
            MidiMsg::ChannelMode { channel: _, msg } => match msg {
                ChannelModeMsg::AllNotesOff => self.release_all(),
                ChannelModeMsg::AllSoundOff => self.all_sounds_off(),
                _ => {}
            },
            MidiMsg::SystemRealTime { msg } => match msg {
                SystemRealTimeMsg::SystemReset => return Some(RelayedMessage::SystemReset),
                _ => {}
            },
            _ => {}
        }
        None
    }

    fn find_next_state(&mut self) -> usize {
        for i in self.next.iter() {
            if self.recent_pitches[i.a()].is_none() {
                return self.claim_state(i);
            }
        }
        self.pitch2state[self.recent_pitches[self.next.a()].unwrap() as usize] = None;
        self.release(self.next.a());
        self.claim_state(self.next)
    }

    fn claim_state(&mut self, state: ModNumC<usize, N>) -> usize {
        let next = state.a();
        self.next = state + 1;
        next
    }

    fn on(&mut self, pitch: u8, velocity: u8) {
        self.master_volume.set_value(1.0);
        let selected = self.find_next_state();
        self.states[selected].on(pitch, velocity);
        self.pitch2state[pitch as usize] = Some(selected);
        self.recent_pitches[selected] = Some(pitch);
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
        self.all_sounds_off();
        self.synth_func = new_synth;
    }

    fn bend(&mut self, bend: u16) {
        for state in self.states.iter_mut() {
            state.bend(bend);
        }
    }

    fn sound_at(&self, i: usize) -> Box<dyn AudioUnit> {
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

    fn all_sounds_off(&mut self) {
        self.master_volume.set_value(0.0);
    }
}
