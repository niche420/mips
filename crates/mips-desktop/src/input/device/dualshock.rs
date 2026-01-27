use mips_core::input::{ButtonQueue, DeviceType};
use crate::input::{Device, RawInput};

pub struct ButtonIdx(pub u8);

pub struct DualShock {
    button_queue: ButtonQueue
}

impl DualShock {
    pub fn new() -> Self {
        DualShock {
            button_queue: Vec::new()
        }
    }
}

impl Device for DualShock {
    fn device_type(&self) -> DeviceType {
        DeviceType::DualShock
    }
}

impl RawInput for ButtonIdx {
    fn device_type() -> DeviceType {
        DeviceType::DualShock
    }

    fn as_string(&self) -> String {
        self.0.to_string()
    }
}