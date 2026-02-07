use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig, SampleRate};
use tracing::{info, error, warn};

pub struct AudioManager {
    _stream: Stream,
    buffer: Arc<Mutex<Vec<f32>>>, // Use f32 for better quality
    volume: Arc<Mutex<f32>>,
    device_sample_rate: u32,
}

impl AudioManager {
    pub fn new() -> anyhow::Result<Self> {
        info!("Initializing audio");

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No audio output device available"))?;

        info!("Audio device: {}", device.name()?);

        // Try to get 44100 Hz support, otherwise use device default
        let config = match device.supported_output_configs() {
            Ok(mut configs) => {
                // Try to find 44100 Hz support
                if let Some(cfg) = configs.find(|c| {
                    c.min_sample_rate().0 <= 44100 && c.max_sample_rate().0 >= 44100
                }) {
                    info!("Using native 44.1kHz");
                    StreamConfig {
                        channels: 2,
                        sample_rate: SampleRate(44100),
                        buffer_size: cpal::BufferSize::Default,
                    }
                } else {
                    // Use device default
                    let default = device.default_output_config()?;
                    warn!("Device doesn't support 44.1kHz, using {} Hz", default.sample_rate().0);
                    StreamConfig {
                        channels: 2,
                        sample_rate: default.sample_rate(),
                        buffer_size: cpal::BufferSize::Default,
                    }
                }
            }
            Err(_) => {
                let default = device.default_output_config()?;
                StreamConfig {
                    channels: 2,
                    sample_rate: default.sample_rate(),
                    buffer_size: cpal::BufferSize::Default,
                }
            }
        };

        let device_sample_rate = config.sample_rate.0;
        info!("Audio config: {} Hz, {} channels", device_sample_rate, config.channels);

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = Arc::clone(&buffer);

        let volume = Arc::new(Mutex::new(1.0f32));
        let volume_clone = Arc::clone(&volume);

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut buf = buffer_clone.lock().unwrap();
                let vol = *volume_clone.lock().unwrap();

                let samples_to_copy = data.len().min(buf.len());

                // Copy samples and apply volume
                for i in 0..samples_to_copy {
                    data[i] = buf[i] * vol;
                }

                // Clear consumed samples
                buf.drain(..samples_to_copy);

                // Fill remaining with silence if needed
                for i in samples_to_copy..data.len() {
                    data[i] = 0.0;
                }
            },
            |err| {
                error!("Audio stream error: {}", err);
            },
            None,
        )?;

        stream.play()?;
        info!("Audio stream started");

        Ok(Self {
            _stream: stream,
            buffer,
            volume,
            device_sample_rate,
        })
    }

    pub fn queue_samples(&self, samples: &[i16]) {
        if samples.is_empty() {
            return;
        }

        // Convert i16 to f32 and resample if needed
        let samples_f32: Vec<f32> = samples.iter()
            .map(|&s| s as f32 / 32768.0) // Convert to -1.0 to 1.0 range
            .collect();

        let resampled = if self.device_sample_rate != 44100 {
            // Simple but higher quality linear resampling
            resample_linear(&samples_f32, 44100, self.device_sample_rate)
        } else {
            samples_f32
        };

        let mut buffer = self.buffer.lock().unwrap();
        buffer.extend_from_slice(&resampled);

        // Prevent buffer from growing too large
        let max_size = (self.device_sample_rate as usize) * 2; // 2 for stereo
        if buffer.len() > max_size {
            let overflow = buffer.len() - max_size;
            buffer.drain(..overflow);
            warn!("Audio buffer overflow, dropped {} samples", overflow);
        }
    }

    pub fn set_volume(&self, volume: f32) {
        *self.volume.lock().unwrap() = volume.clamp(0.0, 1.0);
    }
}

/// High-quality linear resampling
fn resample_linear(input: &[f32], input_rate: u32, output_rate: u32) -> Vec<f32> {
    if input_rate == output_rate {
        return input.to_vec();
    }

    let ratio = output_rate as f64 / input_rate as f64;
    let output_len = (input.len() as f64 * ratio).ceil() as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_pos = i as f64 / ratio;
        let src_index = src_pos.floor() as usize;
        let frac = src_pos - src_index as f64;

        if src_index + 1 < input.len() {
            // Linear interpolation
            let sample0 = input[src_index] as f64;
            let sample1 = input[src_index + 1] as f64;
            let interpolated = sample0 + (sample1 - sample0) * frac;
            output.push(interpolated as f32);
        } else if src_index < input.len() {
            output.push(input[src_index]);
        } else {
            output.push(0.0);
        }
    }

    output
}