use std::error::Error;
use std::path::Path;
use cdimage::cue::Cue;
use log::{error, info};
use crate::ps1::mem_card::MemoryCardFile;
use crate::ps1::psx::bus::Bus;
use crate::ps1::util::ds::box_slice::BoxSlice;
use crate::ps1::util::fs::sys_dir::{SearchFor, SysDir};
use crate::error::MipsResult;
use crate::input::{ButtonQueue, DeviceType};
use crate::ps1::psx::bios::bios::Bios;
use crate::ps1::psx::cd::disc::Disc;
use crate::ps1::psx::exe::Exe;
use crate::ps1::psx::graphics::rasterizer::handle::Frame;
use psx::pad_memcard::gamepad::{DigitalPad, DualShock};
use crate::ps1::util::fs::file::bin;

mod hash;
mod psx;
mod settings;
mod util;
mod error;
mod mem_card;
mod bitwise;

pub use error::Ps1Error;
pub use psx::graphics::rasterizer::handle::Frame as Ps1Frame;

use crate::{gfx, Console};
use crate::ps1::psx::cd::CDC_ROM_SIZE;
use crate::ps1::psx::pad_memcard::{DeviceInterface, DisconnectedDevice};
use crate::ps1::settings::Ps1Settings;

pub struct Ps1 {
    bus: Box<Bus>,
    settings: Ps1Settings,
    memcard_files: BoxSlice<MemoryCardFile, 2>,
    sys_dir: SysDir
}

impl Ps1 {
    pub fn new(sys_dir: &Path, game_path: Option<&str>) -> MipsResult<Ps1> {
        let sys_dir = SysDir::new(sys_dir);

        let mut cdc_firmware = {
            let cdc_firmware_path = sys_dir.search(SearchFor::CdcFirmware)?;
            open_cdc_firmware(cdc_firmware_path.as_path())?
        };

        //let test_exe = {
        //    let exe_path = sys_dir.search(SearchFor::Executables)?;
        //    let test_exe_path = exe_path.join("psxtest_cpu.exe");
        //    open_exe(test_exe_path.as_path())?
        //};

        let bios = {
            let bios_path = sys_dir.search(SearchFor::Bios)?;
            open_bios(bios_path.as_path())?
        };

        let disc = {
            match game_path {
                Some(game_path) => {
                    let games_path = sys_dir.search(SearchFor::Games)?;
                    let disc_path = games_path.join(game_path);
                    Some(open_disc(disc_path.as_path())?)
                },
                None => None
            }
        };

        Ok(Ps1 {
            bus: Box::new(Bus::new(bios, *cdc_firmware, disc)?),
            settings: Ps1Settings::default(),
            memcard_files: BoxSlice::from_vec(vec![MemoryCardFile::dummy(), MemoryCardFile::dummy()]),
            sys_dir
        })
    }

    pub fn insert_disc(&mut self, disc_path: &str) -> MipsResult<()> {
        let disc = {
            let games_path = self.sys_dir.search(SearchFor::Games)?;
            let disc_path = games_path.join(disc_path);
            open_disc(disc_path.as_path())?
        };

        self.bus.insert_disc(disc);
        Ok(())
    }

    pub fn poll_mem_cards(&mut self) {
        let mut memory_cards = self.bus.pad_memcard.memory_cards_mut();
        for (file, mc) in self.memcard_files.iter_mut().zip(memory_cards.iter_mut()) {
            let device = mc.device_mut();

            device.new_frame();
            file.maybe_dump(device);
        }
    }

    pub fn poll_gamepads(&mut self, button_states: ButtonQueue) {
        // Refresh pads
        let gamepads = self.bus.pad_memcard.gamepads_mut();

        let device = gamepads[0].device_mut();

        for (state, button) in button_states.iter() {
            device.set_button_state(*button, *state);
        }
    }
}

impl Console for Ps1 {
    fn update(&mut self) {
        self.bus.update();
    }

    fn clear_audio_samples(&mut self) {
        self.bus.clear_audio_samples()
    }

    fn connect_device(&mut self, port: usize, mut device_type: DeviceType) {
        let gamepads = self.bus.pad_memcard.gamepads_mut();

        let new_pad: Box<dyn DeviceInterface> = match device_type {
            DeviceType::Unknown => Box::new(DisconnectedDevice),
            DeviceType::Keyboard => Box::new(DigitalPad::new()),
            DeviceType::DualShock => Box::new(DualShock::new()),
            _ => {
                error!(
                "Received bogus controller config for port {}: {:?}.\
                               Disconnecting it",
                port, device_type
                );
                device_type = DeviceType::Unknown;
                Box::new(DisconnectedDevice)
            }
        };

        info!("New controller on port {}: {}", port, new_pad.description());

        gamepads[port].connect_device(new_pad);
    }

    fn get_frame(&mut self) -> Option<gfx::CpuFrame> {
        match self.bus.take_frame() {
            Some(frame) => Some(gfx::CpuFrame::from(frame)),
            None => None
        }
    }

    fn get_audio_samples(&mut self) -> &[i16] {
        self.bus.get_audio_samples()
    }

    fn handle_inputs(&mut self, inputs: ButtonQueue) {
        let gamepads = self.bus.pad_memcard.gamepads_mut();

        let device = gamepads[0].device_mut();

        for (state, button) in inputs.iter() {
            device.set_button_state(*button, *state);
        }
    }

    fn refresh_devices(&mut self) {
        // Refresh pads
        let mut gamepads = self.bus.pad_memcard.gamepads_mut();
        for gp in gamepads.iter_mut() {
            let device = gp.device_mut();
            device.new_frame();
        }
    }
}

fn open_bios(bios_path: &Path) -> MipsResult<Bios> {
    let rom = bin::from_file(bios_path)?;
    let bios = Bios::new(rom)?;
    Ok(bios)
}

/// Attempt to find the CDC firmware in the system directory
fn open_cdc_firmware(cdc_firmware_path: &Path) -> MipsResult<BoxSlice<u8, CDC_ROM_SIZE>> {
    let rom = bin::from_file(cdc_firmware_path)?;
    Ok(rom)
}

fn open_disc(disc_path: &Path) -> MipsResult<Disc> {
    let path = disc_path;

    let disc = if path.extension().and_then(|ext| ext.to_str()) == Some("cue") {
        Cue::new(path)
    } else {
        Cue::new_from_zip(path)
    }.unwrap();

    let disc = Disc::new(Box::new(disc))?;

    let serial = disc.serial_number();
    let region = disc.region();

    info!("Disc serial number: {}", serial);
    info!("Detected disc region: {:?}", region);

    Ok(disc)
}

fn open_exe(path: &Path) -> MipsResult<Exe> {
    let exe = Exe::new(path);

    exe
}