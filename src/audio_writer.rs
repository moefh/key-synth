use std::result::Result;
use std::error::Error;
use std::sync::{Arc, Mutex};
use super::synth::SynthPlayer;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[allow(dead_code)]
pub struct AudioWriter {
    host: cpal::Host,
    device: cpal::Device,
    stream: cpal::Stream,
}

impl AudioWriter {
    fn read_supported_output_configs(device: &cpal::Device) -> String {
        let configs_iter = device.supported_output_configs();
        match configs_iter {
            Ok(configs) => {
                let mut s = String::new();
                for config in configs {
                    s.push_str(&format!("-> {:?}\n", config));
                }
                s
            }
            Err(e) => e.to_string()
        }
    }

    pub fn start(sample_rate: u32, buffer_size: u32, player: Arc<Mutex<SynthPlayer>>) -> Result<Self, Box<dyn Error>> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or_else(|| {
            std::io::Error::other("can't open audio output device")
        })?;
        let supported_config_range = device.supported_output_configs()?.find(|range| {
            if matches!(range.sample_format(), cpal::SampleFormat::I16) &&
                range.channels() == 2 &&
                range.min_sample_rate().0 <= sample_rate &&
                range.max_sample_rate().0 >= sample_rate &&
                let cpal::SupportedBufferSize::Range{ min: min_buffer_size, max: max_buffer_size } = range.buffer_size() &&
                *min_buffer_size <= buffer_size &&
                *max_buffer_size >= buffer_size {
                    true
                } else {
                    false
                }
        });
        let mut config = supported_config_range.ok_or_else(|| {
            std::io::Error::other(format!("no suitable config found.\nSupported configs:\n{}",
                                          Self::read_supported_output_configs(&device)))
        })?.try_with_sample_rate(cpal::SampleRate(sample_rate)).ok_or_else(|| {
            std::io::Error::other("buffer size not supported")
        })?.config();
        config.buffer_size = cpal::BufferSize::Fixed(buffer_size);

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                for spl in data.iter_mut() {
                    *spl = 0;
                }
                let mut player = player.lock().unwrap();
                player.gen_samples(data);
            },
            move |err| { println!("CPAL error: {}", err); },
            None)?;
        stream.play()?;

        Ok(AudioWriter {
            host,
            device,
            stream,
        })
    }
}
