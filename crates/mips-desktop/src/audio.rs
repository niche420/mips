use std::collections::VecDeque;
use std::ops::Deref;
use sdl3::audio::{AudioCallback, AudioFormat, AudioFormatNum, AudioSpec, AudioStream, AudioStreamWithCallback};

#[derive(Clone)]
pub struct Device {
    device: sdl3::audio::AudioDevice
}

impl Device {
    pub fn resume(&mut self) {
        self.device.resume();
    }
}

impl From<sdl3::AudioSubsystem> for Device {
    fn from(audio: sdl3::AudioSubsystem) -> Self {
        Device {
            device: audio.default_playback_device()
        }
    }
}

impl Deref for Device {
    type Target = sdl3::audio::AudioDevice;
    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

pub struct StreamWithCallback {
    stream: AudioStreamWithCallback<StreamCallback>
}

impl StreamWithCallback {
    pub fn resume(&mut self) {
        self.stream.resume().unwrap();
    }

    pub fn pause(&mut self) {
        self.stream.pause().unwrap();
    }

    pub fn enqueue(&mut self, data: &[i16]) {
        {
            if let Some(mut guard) = self.stream.lock() {
                guard.queue.push_back(data.to_vec());
            }
        }
    }
}

struct StreamCallback {
    queue: VecDeque<Vec<i16>>
}

impl<Channel: AudioFormatNum> AudioCallback<Channel> for StreamCallback {
    fn callback(&mut self, stream: &mut AudioStream, requested: i32) {
        if let Some(sample) = self.queue.pop_front() {
            stream.put_data_i16(sample.as_slice()).unwrap();
        }
    }
}

impl From<Device> for StreamWithCallback {
    fn from(dev: Device) -> Self {
        let spec = AudioSpec {
            // The PSX SPU runs at 44.1 kHz
            freq: Some(44100),
            channels: Some(2),
            format: Some(AudioFormat::S16LE),
        };

        let stream_callback = StreamCallback {
            queue: VecDeque::new()
        };
        let stream_with_callback = dev.deref().open_playback_stream_with_callback::
        <StreamCallback, i16>(&spec, stream_callback).unwrap();

        Self {
            stream: stream_with_callback
        }
    }
}
