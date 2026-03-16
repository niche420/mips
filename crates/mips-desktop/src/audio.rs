use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player};
use rodio::buffer::SamplesBuffer;
use rodio::nz;
use tracing::info;

pub struct AudioManager {
    _handle: MixerDeviceSink,
    player: Player,
}

impl AudioManager {
    pub fn new() -> anyhow::Result<Self> {
        let handle = DeviceSinkBuilder::open_default_sink()
            .map_err(|e| anyhow::anyhow!("Failed to open audio: {}", e))?;
        let player = Player::connect_new(&handle.mixer());

        info!("Audio initialized");

        Ok(Self {
            _handle: handle,
            player,
        })
    }

    pub fn enqueue(&self, samples: &[i16]) {
        if samples.is_empty() {
            return;
        }
        let samples_f32: Vec<f32> = samples.iter()
            .map(|&s| s as f32 / 32768.0)
            .collect();
        let buf = SamplesBuffer::new(nz!(2u16), nz!(44100u32), samples_f32);
        self.player.append(buf);
    }

    pub fn set_volume(&self, volume: f32) {
        self.player.set_volume(volume.clamp(0.0, 1.0));
    }
}