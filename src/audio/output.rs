use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

pub type SharedAudioBuffer = Arc<Mutex<VecDeque<f32>>>;

pub struct CpalOutput {
    device: cpal::Device,
    stream_config: cpal::StreamConfig,
    stream: Option<cpal::Stream>,
}

impl CpalOutput {
    pub fn new(sample_rate: u32, channels: u16) -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No output device found"))?;

        // Try to find a supported configuration
        let supported_configs = device
            .supported_output_configs()
            .map_err(|e| anyhow::anyhow!("Failed to get supported configs: {}", e))?;

        let mut best_config = None;
        let mut best_rate_distance = u32::MAX;
        for config in supported_configs {
            eprintln!(
                "[CPAL] Supported config: {:?}, channels={}, rate={:?}-{:?}",
                config.sample_format(),
                config.channels(),
                config.min_sample_rate(),
                config.max_sample_rate()
            );

            if config.sample_format() == cpal::SampleFormat::F32 && config.channels() == channels {
                let min_rate = config.min_sample_rate().0;
                let max_rate = config.max_sample_rate().0;
                let selected_rate = sample_rate.clamp(min_rate, max_rate);
                let distance = selected_rate.abs_diff(sample_rate);

                if distance < best_rate_distance {
                    best_rate_distance = distance;
                    best_config = Some(config.with_sample_rate(cpal::SampleRate(selected_rate)));
                }
            }
        }

        let supported_config = best_config.ok_or_else(|| {
            anyhow::anyhow!(
                "No supported f32 output config found for {} channels",
                channels
            )
        })?;

        eprintln!(
            "[CPAL] Using config: channels={}, rate={}",
            supported_config.channels(),
            supported_config.sample_rate().0
        );

        let stream_config: cpal::StreamConfig = supported_config.into();

        Ok(Self {
            device,
            stream_config,
            stream: None,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.stream_config.sample_rate.0
    }

    pub fn channels(&self) -> u16 {
        self.stream_config.channels
    }

    pub fn start(&mut self, buffer: SharedAudioBuffer) -> anyhow::Result<()> {
        let stream = self.device.build_output_stream(
            &self.stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                data.fill(0.0);

                if let Ok(mut buf) = buffer.try_lock() {
                    for sample in data {
                        let Some(buffered_sample) = buf.pop_front() else {
                            break;
                        };
                        *sample = buffered_sample;
                    }
                }
            },
            |err| {
                eprintln!("[ERROR] CpalOutput stream error: {}", err);
            },
            None,
        )?;

        stream.play()?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn pause(&self) -> anyhow::Result<()> {
        if let Some(stream) = &self.stream {
            stream.pause()?;
        }
        Ok(())
    }

    pub fn resume(&self) -> anyhow::Result<()> {
        if let Some(stream) = &self.stream {
            stream.play()?;
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        self.stream = None;
    }
}
