use std::collections::HashMap;
use egui::Key;
use mips_core::input::{Button, ButtonQueue, ButtonState};
use gilrs::{Gilrs, Button as GilrsButton, EventType};
use tracing::info;

pub struct InputManager {
    // Store key states for change detection
    key_states: HashMap<Key, bool>,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            key_states: HashMap::new(),
        }
    }

    pub fn poll_input(&mut self, ctx: &egui::Context, bindings: &HashMap<Key, Button>) -> ButtonQueue {
        let mut queue = Vec::new();

        ctx.input(|i| {
            // Check all bound keys
            for (key, button) in bindings.iter() {
                let is_down = i.key_down(*key);
                let was_down = self.key_states.get(key).copied().unwrap_or(false);

                if is_down != was_down {
                    self.key_states.insert(*key, is_down);

                    let state = if is_down {
                        ButtonState::Pressed
                    } else {
                        ButtonState::Released
                    };
                    queue.push((state, *button));
                }
            }
        });

        queue
    }
}

pub struct GamepadManager {
    pub(crate) gilrs: Option<Gilrs>,
}

impl GamepadManager {
    pub fn new() -> Self {
        let gilrs = match Gilrs::new() {
            Ok(gilrs) => {
                info!("Gamepad support initialized");
                Some(gilrs)
            }
            Err(e) => {
                tracing::warn!("Failed to initialize gamepad support: {}", e);
                None
            }
        };

        Self { gilrs }
    }

    pub fn poll_gamepad(&mut self, button_queue: &mut ButtonQueue, bindings: &HashMap<GilrsButton, Button>) {
        let Some(gilrs) = &mut self.gilrs else {
            return;
        };

        // Process gamepad events
        while let Some(event) = gilrs.next_event() {
            match event.event {
                EventType::ButtonPressed(gilrs_button, _) => {
                    if let Some(ps_button) = bindings.get(&gilrs_button) {
                        button_queue.push((ButtonState::Pressed, *ps_button));
                    }
                }
                EventType::ButtonReleased(gilrs_button, _) => {
                    if let Some(ps_button) = bindings.get(&gilrs_button) {
                        button_queue.push((ButtonState::Released, *ps_button));
                    }
                }
                EventType::Connected => {
                    info!("Gamepad connected");
                }
                EventType::Disconnected => {
                    info!("Gamepad disconnected");
                }
                _ => {}
            }
        }
    }
}