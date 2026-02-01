use std::sync::{Arc, Mutex};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use tracing::{info, error};

pub struct AudioManager {
    _stream: Stream,
    buffer: Arc<Mutex<Vec<i16>>>,
    volume: Arc<Mutex<f32>>,
}

impl AudioManager {
    pub fn new() -> anyhow::Result<Self> {
        info!("Initializing audio");

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("No audio output device available"))?;

        info!("Audio device: {}", device.name()?);

        let mut supported_configs_range = device.supported_output_configs()
            .expect("error while querying configs");
        let config = supported_configs_range.next()
            .expect("no supported config?!")
            .with_max_sample_rate().config();

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let buffer_clone = Arc::clone(&buffer);

        let volume = Arc::new(Mutex::new(1.0f32));
        let volume_clone = Arc::clone(&volume);

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                let mut buf = buffer_clone.lock().unwrap();
                let vol = *volume_clone.lock().unwrap();

                let samples_to_copy = data.len().min(buf.len());

                // Copy samples and apply volume
                for i in 0..samples_to_copy {
                    data[i] = (buf[i] as f32 * vol) as i16;
                }

                // Clear consumed samples
                buf.drain(..samples_to_copy);

                // Fill remaining with silence if needed
                for i in samples_to_copy..data.len() {
                    data[i] = 0;
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
        })
    }

    pub fn queue_samples(&self, samples: &[i16]) {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.extend_from_slice(samples);

        // Prevent buffer from growing too large
        const MAX_BUFFER_SIZE: usize = 44100 * 2; // 1 second of stereo audio
        if buffer.len() > MAX_BUFFER_SIZE {
            let overflow = buffer.len() - MAX_BUFFER_SIZE;
            buffer.drain(..overflow);
        }
    }

    pub fn set_volume(&self, volume: f32) {
        *self.volume.lock().unwrap() = volume.clamp(0.0, 1.0);
    }
}