use std::path::Path;
use crate::input::{ButtonQueue, DeviceType};
use crate::ps1::Ps1;

pub mod input;
mod error;

#[cfg(feature = "ps1")]
mod ps1;
mod gfx;

pub use error::MipsError;
use crate::error::MipsResult;
use crate::gfx::CpuFrame;

pub trait Console {
    fn update(&mut self);
    fn get_frame(&mut self) -> Option<CpuFrame>;
    fn get_audio_samples(&mut self) -> &[i16];
    fn clear_audio_samples(&mut self);
    fn connect_device(&mut self, port: usize, device_type: DeviceType);
    fn handle_inputs(&mut self, inputs: ButtonQueue);
    fn refresh_devices(&mut self);
}

pub struct ConsoleManager {
    active: Option<Box<dyn Console>>,
}

impl ConsoleManager {
    pub fn new() -> Self {
        Self { active: None }
    }
    
    pub fn load_game(&mut self, game_dir: &Path, disc: Option<&str>) -> MipsResult<()> {
        self.active = Some(Box::new(Ps1::new(game_dir, disc)?));
        Ok(())
    }

    // Delegate to active console
    pub fn update(&mut self) {
        if let Some(console) = &mut self.active {
            console.update();
        }
    }

    pub fn get_frame(&mut self) -> Option<CpuFrame> {
        self.active.as_mut().and_then(|c| c.get_frame())
    }

    pub fn get_audio_samples(&mut self) -> &[i16] {
        self.active.as_mut()
            .map(|c| c.get_audio_samples())
            .unwrap_or(&[])
    }

    pub fn clear_audio_samples(&mut self) {
        if let Some(console) = &mut self.active {
            console.clear_audio_samples();
        }
    }

    pub fn connect_device(&mut self, port: usize, device: DeviceType) {
        if let Some(console) = &mut self.active {
            console.connect_device(port, device);
        }
    }

    pub fn handle_inputs(&mut self, inputs: ButtonQueue) {
        if let Some(console) = &mut self.active {
            console.handle_inputs(inputs);
        }
    }

    pub fn refresh_devices(&mut self) {
        if let Some(console) = &mut self.active {
            console.refresh_devices();
        }
    }
}