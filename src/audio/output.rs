use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

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

        let stream_config = cpal::StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        Ok(Self {
            device,
            stream_config,
            stream: None,
        })
    }

    pub fn start(&mut self, buffer: Arc<Mutex<Vec<f32>>>) -> anyhow::Result<()> {
        let stream = self.device.build_output_stream(
            &self.stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut buf = buffer.lock().unwrap();
                let len = data.len().min(buf.len());
                for i in 0..len {
                    data[i] = buf[i];
                }
                buf.drain(..len);
                for sample in &mut data[len..] {
                    *sample = 0.0;
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

    pub fn stop(&mut self) {
        self.stream = None;
    }
}
