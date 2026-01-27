use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc::Receiver;
use mips_core::input::{Button, ButtonQueue, ButtonState, DeviceType, InputConfig};
use crate::input::device::InputDevice;

pub mod device;

pub use device::InputDeviceMap;

pub trait RawInput {
    fn device_type() -> DeviceType;
    fn as_string(&self) -> String;
}

pub struct Port {
    input_recv: Option<Receiver<(ButtonState, String)>>,
    btn_map: ButtonMap
}

impl Port {
    pub fn new() -> Self {
        Self {
            input_recv: None,
            btn_map: ButtonMap::new()
        }
    }

    pub fn connect_controller(&mut self, controller: Rc<RefCell<InputDevice>>) {
        let (input_send, input_recv) = std::sync::mpsc::channel();
        self.input_recv = Some(input_recv);
        controller.borrow_mut().add_port_sender(input_send);
    }

    pub fn load_config(&mut self, config: InputConfig) {
        self.btn_map = ButtonMap::from(config);
    }

    pub fn inputs(&self) -> ButtonQueue {
        let mut queue = ButtonQueue::new();
        if let Some(input_recv) = self.input_recv.as_ref() {
            while let Ok((state, input_string)) = input_recv.try_recv() {
                if let Some(button)  = self.btn_map.button(input_string) {
                    queue.push((state, button));
                }
            }
        }

        queue
    }
}

pub struct ButtonMap {
    map: HashMap<String, Button>
}

impl ButtonMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new()
        }
    }

    pub fn button(&self, input: String) -> Option<Button> {
        match self.map.get(&input) {
            Some(btn) => Some(*btn),
            None => None
        }
    }
}

impl From<InputConfig> for ButtonMap {
    fn from(config: InputConfig) -> Self {
        Self {
            map: config.bindings()
        }
    }
}

pub trait Device {
    fn device_type(&self) -> DeviceType;
}