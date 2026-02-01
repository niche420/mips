use std::collections::HashMap;
use egui::Key;
use mips_core::input::{Button, ButtonQueue, ButtonState};
use gilrs::{Gilrs, Button as GilrsButton, EventType};
use tracing::info;

pub struct InputManager {
    // Store key states for direct polling without egui::Context
    key_states: HashMap<String, bool>,
}

impl InputManager {
    pub fn new() -> Self {
        Self {
            key_states: HashMap::new(),
        }
    }

    pub fn poll_input(&mut self, ctx: &egui::Context, button_map: &HashMap<String, Button>) -> ButtonQueue {
        let mut queue = Vec::new();

        // Check keyboard input
        ctx.input(|i| {
            // Arrow keys
            self.check_key(i, Key::ArrowUp, "Up", button_map, &mut queue);
            self.check_key(i, Key::ArrowDown, "Down", button_map, &mut queue);
            self.check_key(i, Key::ArrowLeft, "Left", button_map, &mut queue);
            self.check_key(i, Key::ArrowRight, "Right", button_map, &mut queue);

            // Face buttons
            self.check_key(i, Key::Z, "Z", button_map, &mut queue);
            self.check_key(i, Key::X, "X", button_map, &mut queue);
            self.check_key(i, Key::A, "A", button_map, &mut queue);
            self.check_key(i, Key::S, "S", button_map, &mut queue);

            // Shoulder buttons
            self.check_key(i, Key::Q, "Q", button_map, &mut queue);
            self.check_key(i, Key::W, "W", button_map, &mut queue);
            self.check_key(i, Key::E, "E", button_map, &mut queue);
            self.check_key(i, Key::R, "R", button_map, &mut queue);

            // Start/Select
            self.check_key(i, Key::Enter, "Return", button_map, &mut queue);
            self.check_key(i, Key::Backspace, "Backspace", button_map, &mut queue);
        });

        queue
    }

    pub fn poll_input_direct(&self, button_map: &HashMap<String, Button>) -> ButtonQueue {
        // This is called during emulator updates when we don't have egui::Context
        // Return empty queue as input is already handled via poll_input
        Vec::new()
    }

    fn check_key(
        &mut self,
        input: &egui::InputState,
        key: Key,
        key_name: &str,
        button_map: &HashMap<String, Button>,
        queue: &mut ButtonQueue,
    ) {
        let is_down = input.key_down(key);
        let was_down = self.key_states.get(key_name).copied().unwrap_or(false);

        if is_down != was_down {
            self.key_states.insert(key_name.to_string(), is_down);

            if let Some(button) = button_map.get(key_name) {
                let state = if is_down {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                };
                queue.push((state, *button));
            }
        }
    }
}

pub struct GamepadManager {
    gilrs: Option<Gilrs>,
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

    pub fn poll_gamepad(&mut self, button_queue: &mut ButtonQueue) {
        let Some(gilrs) = &mut self.gilrs else {
            return;
        };

        // Process gamepad events
        while let Some(event) = gilrs.next_event() {
            match event.event {
                EventType::ButtonPressed(button, _) => {
                    if let Some(ps_button) = map_button(button) {
                        button_queue.push((ButtonState::Pressed, ps_button));
                    }
                }
                EventType::ButtonReleased(button, _) => {
                    if let Some(ps_button) = map_button(button) {
                        button_queue.push((ButtonState::Released, ps_button));
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

fn map_button(button: GilrsButton) -> Option<Button> {
    // Map gamepad buttons to PS1 controller buttons
    match button {
        GilrsButton::South => Some(Button::Cross),      // A/Cross
        GilrsButton::East => Some(Button::Circle),      // B/Circle
        GilrsButton::West => Some(Button::Square),      // X/Square
        GilrsButton::North => Some(Button::Triangle),   // Y/Triangle
        GilrsButton::LeftTrigger => Some(Button::L1),
        GilrsButton::RightTrigger => Some(Button::R1),
        GilrsButton::LeftTrigger2 => Some(Button::L2),
        GilrsButton::RightTrigger2 => Some(Button::R2),
        GilrsButton::Select => Some(Button::Select),
        GilrsButton::Start => Some(Button::Start),
        GilrsButton::DPadUp => Some(Button::DUp),
        GilrsButton::DPadDown => Some(Button::DDown),
        GilrsButton::DPadLeft => Some(Button::DLeft),
        GilrsButton::DPadRight => Some(Button::DRight),
        _ => None,
    }
}