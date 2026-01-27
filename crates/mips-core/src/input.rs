mod pad;

use std::collections::HashMap;
use std::path::Path;
use ini::Ini;
use log::warn;
use num_traits::FromPrimitive;

pub use crate::input::pad::{Button, ButtonState};

pub type ButtonQueue = Vec<(ButtonState, Button)>;

#[derive(Hash, Copy, Clone, Eq, PartialEq, Debug)]
pub enum DeviceType {
    Unknown,
    Keyboard,
    DualShock,
}

pub struct InputConfig {
    device_type: DeviceType,
    bindings: HashMap<String, Button>
}

impl InputConfig {
    pub fn write(&self) { todo!() }

    pub fn bindings(&self) -> HashMap<String, Button> {
        self.bindings.clone()
    }
}

impl From<&Path> for InputConfig {
    fn from(path: &Path) -> Self {
        let ini = Ini::load_from_file(path).unwrap();
        let device_type = ini.section(Some("Device")).unwrap().get("Type").unwrap();
        let bindings_sec = ini.section(Some("Bindings")).unwrap();

        let mut device_type = match device_type {
            "Keyboard" => DeviceType::Keyboard,
            "Dualshock" => DeviceType::DualShock,
            _ => {
                warn!("Unknown device type in input config file {}: DeviceType = {}", path.display(), device_type);
                DeviceType::Unknown
            },
        };

        let mut bindings = HashMap::new();
        for (device_input, psx_input) in bindings_sec {
            bindings.insert(device_input.to_string(), Button::from_u32(psx_input.parse::<u32>().unwrap()).unwrap());
        }

        InputConfig {
            device_type,
            bindings
        }
    }
}