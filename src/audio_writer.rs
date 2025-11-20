use std::result::Result;
use std::error::Error;
use std::sync::{Arc, Mutex};
use super::synth::SynthPlayer;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[derive(Clone, Copy)]
pub struct RequestedConfig {
    pub min_sample_rate: u32,
    pub max_sample_rate: u32,
    pub pref_sample_rate: u32,
    pub buffer_size: u32,
    pub num_channels: u16,
}

#[allow(dead_code)]
pub struct AudioWriter {
    host: cpal::Host,
    device: cpal::Device,
    config: cpal::StreamConfig,
    stream: Option<cpal::Stream>,
    pub sample_rate: f32,
    pub num_channels: usize,
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

    fn find_preferred_config(device: &cpal::Device, pref_config: RequestedConfig)
                             -> Result<Option<cpal::SupportedStreamConfigRange>, Box<dyn Error>> {
        let configs = device.supported_output_configs()?.find(|range| {
            let min_sample_rate = pref_config.min_sample_rate.max(range.min_sample_rate().0);
            let max_sample_rate = pref_config.max_sample_rate.min(range.max_sample_rate().0);
            if matches!(range.sample_format(), cpal::SampleFormat::I16) &&
                range.channels() == pref_config.num_channels &&
                min_sample_rate <= max_sample_rate &&
                let cpal::SupportedBufferSize::Range{ min: min_buffer_size, max: max_buffer_size } = range.buffer_size() &&
                *min_buffer_size <= pref_config.buffer_size &&
                *max_buffer_size >= pref_config.buffer_size {
                    true
                } else {
                    false
                }
        });
        Ok(configs)
    }

    fn find_acceptable_config(device: &cpal::Device, pref_config: RequestedConfig)
                              -> Result<Option<cpal::SupportedStreamConfigRange>, Box<dyn Error>> {
        let configs = device.supported_output_configs()?.find(|range| {
            let min_sample_rate = pref_config.min_sample_rate.max(range.min_sample_rate().0);
            let max_sample_rate = pref_config.max_sample_rate.min(range.max_sample_rate().0);
            if matches!(range.sample_format(), cpal::SampleFormat::I16) &&
                range.channels() <= 2 &&
                min_sample_rate <= max_sample_rate &&
                let cpal::SupportedBufferSize::Range{ min: min_buffer_size, max: max_buffer_size } = range.buffer_size() &&
                *min_buffer_size <= pref_config.buffer_size &&
                *max_buffer_size >= pref_config.buffer_size {
                    true
                } else {
                    false
                }
        });
        Ok(configs)
    }

    pub fn init(pref_config: RequestedConfig) -> Result<Self, Box<dyn Error>> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or_else(|| {
            std::io::Error::other("can't open audio output device")
        })?;
        let config_range = match Self::find_preferred_config(&device, pref_config)? {
            Some(config_range) => Some(config_range),
            None => Self::find_acceptable_config(&device, pref_config)?,
        };
        let config_range = config_range.ok_or_else(|| {
            std::io::Error::other(format!("no suitable config found.\nSupported configs:\n{}",
                                          Self::read_supported_output_configs(&device)))
        })?;
        let min_sample_rate = config_range.min_sample_rate().0;
        let max_sample_rate = config_range.max_sample_rate().0;
        let sample_rate = pref_config.pref_sample_rate.clamp(min_sample_rate, max_sample_rate);
        let mut config = config_range.try_with_sample_rate(cpal::SampleRate(sample_rate)).ok_or_else(|| {
            std::io::Error::other("sample rate not supported")
        })?.config();
        config.buffer_size = cpal::BufferSize::Fixed(pref_config.buffer_size);

        let sample_rate = config.sample_rate.0 as f32;
        let num_channels = config.channels as usize;
        Ok(AudioWriter {
            host,
            device,
            config,
            sample_rate,
            num_channels,
            stream: None,
        })
    }

    pub fn start(&mut self, player: Arc<Mutex<SynthPlayer>>) -> Result<(), Box<dyn Error>> {
        let stream = self.device.build_output_stream(
            &self.config,
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
        self.stream = Some(stream);
        Ok(())
    }
}
