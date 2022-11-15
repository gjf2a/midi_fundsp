use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Sample, SampleFormat, StreamConfig};
use fundsp::hacker::*;
use fundsp::prelude::{AudioUnit64};

fn main() {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("failed to find a default output device");
    let config = device.default_output_config().unwrap();
    match config.sample_format() {
        SampleFormat::F32 => run_synth::<f32>(device, config.into()),
        SampleFormat::I16 => run_synth::<i16>(device, config.into()),
        SampleFormat::U16 => run_synth::<u16>(device, config.into()),
    }
}

fn sounder(pitch: u8, velocity: u8) -> Box<dyn AudioUnit64> {
    Box::new(constant(midi_hz(pitch as f64)) >> triangle() * (velocity as f64 / 127.0))
}

fn run_synth<T: Sample>(
    device: Device,
    config: StreamConfig,
) {
    let sample_rate = config.sample_rate.0 as f64;
    let mut sound = Net64::wrap(sounder(60, 100));
    for i in [64, 67, 71, 74, 78, 81] {
        sound = Net64::bin_op(sound, Net64::wrap(sounder(i, i)), FrameAdd::new());
    }
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