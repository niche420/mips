use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use mips_core::input::Button;
use egui::Key;
use gilrs::Button as GilrsButton;
use anyhow::Result;
use tracing::{info, warn};

const CONFIG_DIR: &str = "config";
const SETTINGS_FILE: &str = "settings.toml";
const KEYBOARD_BINDINGS_FILE: &str = "keyboard_bindings.toml";
const GAMEPAD_BINDINGS_FILE: &str = "gamepad_bindings.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub video: VideoSettings,
    pub audio: AudioSettings,
    pub system: SystemSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSettings {
    pub vsync: bool,
    pub bilinear_filter: bool,
    pub window_width: u32,
    pub window_height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    pub volume: f32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSettings {
    pub fast_boot: bool,
    pub auto_save_state: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            video: VideoSettings {
                vsync: true,
                bilinear_filter: false,
                window_width: 1280,
                window_height: 720,
            },
            audio: AudioSettings {
                volume: 1.0,
                enabled: true,
            },
            system: SystemSettings {
                fast_boot: false,
                auto_save_state: true,
            },
        }
    }
}

/// Keyboard bindings - maps egui Key to PS1 Button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardBindings {
    #[serde(with = "keyboard_map")]
    pub bindings: HashMap<Key, Button>,
}

impl Default for KeyboardBindings {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // D-Pad
        bindings.insert(Key::ArrowUp, Button::DUp);
        bindings.insert(Key::ArrowDown, Button::DDown);
        bindings.insert(Key::ArrowLeft, Button::DLeft);
        bindings.insert(Key::ArrowRight, Button::DRight);

        // Face buttons
        bindings.insert(Key::Z, Button::Cross);
        bindings.insert(Key::X, Button::Circle);
        bindings.insert(Key::A, Button::Square);
        bindings.insert(Key::S, Button::Triangle);

        // Shoulder buttons
        bindings.insert(Key::Q, Button::L1);
        bindings.insert(Key::W, Button::R1);
        bindings.insert(Key::E, Button::L2);
        bindings.insert(Key::R, Button::R2);

        // Start/Select
        bindings.insert(Key::Enter, Button::Start);
        bindings.insert(Key::Backspace, Button::Select);

        Self { bindings }
    }
}

/// Gamepad bindings - maps gilrs Button to PS1 Button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamepadBindings {
    #[serde(with = "gamepad_map")]
    pub bindings: HashMap<GilrsButton, Button>,
}

impl Default for GamepadBindings {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // Face buttons (Xbox/standard layout)
        bindings.insert(GilrsButton::South, Button::Cross);      // A/Cross
        bindings.insert(GilrsButton::East, Button::Circle);      // B/Circle
        bindings.insert(GilrsButton::West, Button::Square);      // X/Square
        bindings.insert(GilrsButton::North, Button::Triangle);   // Y/Triangle

        // Shoulder buttons
        bindings.insert(GilrsButton::LeftTrigger, Button::L1);
        bindings.insert(GilrsButton::RightTrigger, Button::R1);
        bindings.insert(GilrsButton::LeftTrigger2, Button::L2);
        bindings.insert(GilrsButton::RightTrigger2, Button::R2);

        // Start/Select
        bindings.insert(GilrsButton::Select, Button::Select);
        bindings.insert(GilrsButton::Start, Button::Start);

        // D-Pad
        bindings.insert(GilrsButton::DPadUp, Button::DUp);
        bindings.insert(GilrsButton::DPadDown, Button::DDown);
        bindings.insert(GilrsButton::DPadLeft, Button::DLeft);
        bindings.insert(GilrsButton::DPadRight, Button::DRight);

        Self { bindings }
    }
}

pub struct ConfigManager {
    config_dir: PathBuf,
    pub settings: AppSettings,
    pub keyboard_bindings: KeyboardBindings,
    pub gamepad_bindings: GamepadBindings,
}

impl ConfigManager {
    pub fn new() -> Result<Self> {
        let config_dir = PathBuf::from(CONFIG_DIR);

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
            info!("Created config directory: {}", config_dir.display());
        }

        let mut manager = Self {
            config_dir,
            settings: AppSettings::default(),
            keyboard_bindings: KeyboardBindings::default(),
            gamepad_bindings: GamepadBindings::default(),
        };

        // Load existing configs or create defaults
        manager.load_or_create_defaults()?;

        Ok(manager)
    }

    fn load_or_create_defaults(&mut self) -> Result<()> {
        // Load settings
        let settings_path = self.config_dir.join(SETTINGS_FILE);
        if settings_path.exists() {
            match fs::read_to_string(&settings_path) {
                Ok(content) => {
                    match toml::from_str(&content) {
                        Ok(settings) => {
                            self.settings = settings;
                            info!("Loaded settings from {}", settings_path.display());
                        }
                        Err(e) => {
                            warn!("Failed to parse settings: {}. Using defaults.", e);
                            self.save_settings()?;
                        }
                    }
                }
                Err(e) => warn!("Failed to read settings: {}. Using defaults.", e),
            }
        } else {
            info!("No settings file found, creating default");
            self.save_settings()?;
        }

        // Load keyboard bindings
        let kb_path = self.config_dir.join(KEYBOARD_BINDINGS_FILE);
        if kb_path.exists() {
            match fs::read_to_string(&kb_path) {
                Ok(content) => {
                    match toml::from_str(&content) {
                        Ok(bindings) => {
                            self.keyboard_bindings = bindings;
                            info!("Loaded keyboard bindings from {}", kb_path.display());
                        }
                        Err(e) => {
                            warn!("Failed to parse keyboard bindings: {}. Using defaults.", e);
                            self.save_keyboard_bindings()?;
                        }
                    }
                }
                Err(e) => warn!("Failed to read keyboard bindings: {}. Using defaults.", e),
            }
        } else {
            info!("No keyboard bindings file found, creating default");
            self.save_keyboard_bindings()?;
        }

        // Load gamepad bindings
        let gp_path = self.config_dir.join(GAMEPAD_BINDINGS_FILE);
        if gp_path.exists() {
            match fs::read_to_string(&gp_path) {
                Ok(content) => {
                    match toml::from_str(&content) {
                        Ok(bindings) => {
                            self.gamepad_bindings = bindings;
                            info!("Loaded gamepad bindings from {}", gp_path.display());
                        }
                        Err(e) => {
                            warn!("Failed to parse gamepad bindings: {}. Using defaults.", e);
                            self.save_gamepad_bindings()?;
                        }
                    }
                }
                Err(e) => warn!("Failed to read gamepad bindings: {}. Using defaults.", e),
            }
        } else {
            info!("No gamepad bindings file found, creating default");
            self.save_gamepad_bindings()?;
        }

        Ok(())
    }

    pub fn save_settings(&self) -> Result<()> {
        let settings_path = self.config_dir.join(SETTINGS_FILE);
        let content = toml::to_string_pretty(&self.settings)?;
        fs::write(&settings_path, content)?;
        info!("Saved settings to {}", settings_path.display());
        Ok(())
    }

    pub fn save_keyboard_bindings(&self) -> Result<()> {
        let kb_path = self.config_dir.join(KEYBOARD_BINDINGS_FILE);
        let content = toml::to_string_pretty(&self.keyboard_bindings)?;
        fs::write(&kb_path, content)?;
        info!("Saved keyboard bindings to {}", kb_path.display());
        Ok(())
    }

    pub fn save_gamepad_bindings(&self) -> Result<()> {
        let gp_path = self.config_dir.join(GAMEPAD_BINDINGS_FILE);
        let content = toml::to_string_pretty(&self.gamepad_bindings)?;
        fs::write(&gp_path, content)?;
        info!("Saved gamepad bindings to {}", gp_path.display());
        Ok(())
    }

    pub fn reset_to_defaults(&mut self) -> Result<()> {
        self.settings = AppSettings::default();
        self.keyboard_bindings = KeyboardBindings::default();
        self.gamepad_bindings = GamepadBindings::default();

        self.save_settings()?;
        self.save_keyboard_bindings()?;
        self.save_gamepad_bindings()?;

        info!("Reset all config to defaults");
        Ok(())
    }
}

// Custom serialization for HashMap<Key, Button>
mod keyboard_map {
    use super::*;
    use serde::{Deserializer, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S>(map: &HashMap<Key, Button>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut s = serializer.serialize_map(Some(map.len()))?;
        for (key, button) in map {
            s.serialize_entry(&key_to_string(key), &button_to_string(button))?;
        }
        s.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<Key, Button>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map: HashMap<String, String> = HashMap::deserialize(deserializer)?;
        let mut result = HashMap::new();

        for (key_str, button_str) in map {
            if let (Some(key), Some(button)) = (string_to_key(&key_str), string_to_button(&button_str)) {
                result.insert(key, button);
            }
        }

        Ok(result)
    }
}

// Custom serialization for HashMap<GilrsButton, Button>
mod gamepad_map {
    use super::*;
    use serde::{Deserializer, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S>(map: &HashMap<GilrsButton, Button>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeMap;
        let mut s = serializer.serialize_map(Some(map.len()))?;
        for (gilrs_button, button) in map {
            s.serialize_entry(&gilrs_button_to_string(gilrs_button), &button_to_string(button))?;
        }
        s.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<GilrsButton, Button>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let map: HashMap<String, String> = HashMap::deserialize(deserializer)?;
        let mut result = HashMap::new();

        for (gilrs_str, button_str) in map {
            if let (Some(gilrs_button), Some(button)) = (string_to_gilrs_button(&gilrs_str), string_to_button(&button_str)) {
                result.insert(gilrs_button, button);
            }
        }

        Ok(result)
    }
}

// Helper functions for Key serialization
fn key_to_string(key: &Key) -> String {
    format!("{:?}", key)
}

fn string_to_key(s: &str) -> Option<Key> {
    match s {
        "ArrowUp" => Some(Key::ArrowUp),
        "ArrowDown" => Some(Key::ArrowDown),
        "ArrowLeft" => Some(Key::ArrowLeft),
        "ArrowRight" => Some(Key::ArrowRight),
        "Enter" => Some(Key::Enter),
        "Backspace" => Some(Key::Backspace),
        "Space" => Some(Key::Space),
        "A" => Some(Key::A),
        "B" => Some(Key::B),
        "C" => Some(Key::C),
        "D" => Some(Key::D),
        "E" => Some(Key::E),
        "F" => Some(Key::F),
        "G" => Some(Key::G),
        "H" => Some(Key::H),
        "I" => Some(Key::I),
        "J" => Some(Key::J),
        "K" => Some(Key::K),
        "L" => Some(Key::L),
        "M" => Some(Key::M),
        "N" => Some(Key::N),
        "O" => Some(Key::O),
        "P" => Some(Key::P),
        "Q" => Some(Key::Q),
        "R" => Some(Key::R),
        "S" => Some(Key::S),
        "T" => Some(Key::T),
        "U" => Some(Key::U),
        "V" => Some(Key::V),
        "W" => Some(Key::W),
        "X" => Some(Key::X),
        "Y" => Some(Key::Y),
        "Z" => Some(Key::Z),
        _ => None,
    }
}

// Helper functions for GilrsButton serialization
fn gilrs_button_to_string(button: &GilrsButton) -> String {
    format!("{:?}", button)
}

fn string_to_gilrs_button(s: &str) -> Option<GilrsButton> {
    match s {
        "South" => Some(GilrsButton::South),
        "East" => Some(GilrsButton::East),
        "North" => Some(GilrsButton::North),
        "West" => Some(GilrsButton::West),
        "LeftTrigger" => Some(GilrsButton::LeftTrigger),
        "RightTrigger" => Some(GilrsButton::RightTrigger),
        "LeftTrigger2" => Some(GilrsButton::LeftTrigger2),
        "RightTrigger2" => Some(GilrsButton::RightTrigger2),
        "Select" => Some(GilrsButton::Select),
        "Start" => Some(GilrsButton::Start),
        "DPadUp" => Some(GilrsButton::DPadUp),
        "DPadDown" => Some(GilrsButton::DPadDown),
        "DPadLeft" => Some(GilrsButton::DPadLeft),
        "DPadRight" => Some(GilrsButton::DPadRight),
        _ => None,
    }
}

// Helper functions for Button serialization
fn button_to_string(button: &Button) -> String {
    match button {
        Button::Select => "Select".to_string(),
        Button::L3 => "L3".to_string(),
        Button::R3 => "R3".to_string(),
        Button::Start => "Start".to_string(),
        Button::DUp => "DUp".to_string(),
        Button::DRight => "DRight".to_string(),
        Button::DDown => "DDown".to_string(),
        Button::DLeft => "DLeft".to_string(),
        Button::L2 => "L2".to_string(),
        Button::R2 => "R2".to_string(),
        Button::L1 => "L1".to_string(),
        Button::R1 => "R1".to_string(),
        Button::Triangle => "Triangle".to_string(),
        Button::Circle => "Circle".to_string(),
        Button::Cross => "Cross".to_string(),
        Button::Square => "Square".to_string(),
        Button::Analog => "Analog".to_string(),
    }
}

fn string_to_button(s: &str) -> Option<Button> {
    match s {
        "Select" => Some(Button::Select),
        "L3" => Some(Button::L3),
        "R3" => Some(Button::R3),
        "Start" => Some(Button::Start),
        "DUp" => Some(Button::DUp),
        "DRight" => Some(Button::DRight),
        "DDown" => Some(Button::DDown),
        "DLeft" => Some(Button::DLeft),
        "L2" => Some(Button::L2),
        "R2" => Some(Button::R2),
        "L1" => Some(Button::L1),
        "R1" => Some(Button::R1),
        "Triangle" => Some(Button::Triangle),
        "Circle" => Some(Button::Circle),
        "Cross" => Some(Button::Cross),
        "Square" => Some(Button::Square),
        "Analog" => Some(Button::Analog),
        _ => None,
    }
}

pub fn button_display_name(button: &Button) -> &'static str {
    match button {
        Button::Select => "Select",
        Button::L3 => "L3",
        Button::R3 => "R3",
        Button::Start => "Start",
        Button::DUp => "D-Pad Up",
        Button::DRight => "D-Pad Right",
        Button::DDown => "D-Pad Down",
        Button::DLeft => "D-Pad Left",
        Button::L2 => "L2",
        Button::R2 => "R2",
        Button::L1 => "L1",
        Button::R1 => "R1",
        Button::Triangle => "Triangle",
        Button::Circle => "Circle",
        Button::Cross => "Cross",
        Button::Square => "Square",
        Button::Analog => "Analog",
    }
}

pub fn key_display_name(key: &Key) -> String {
    match key {
        Key::ArrowUp => "↑".to_string(),
        Key::ArrowDown => "↓".to_string(),
        Key::ArrowLeft => "←".to_string(),
        Key::ArrowRight => "→".to_string(),
        Key::Enter => "Enter".to_string(),
        Key::Backspace => "Backspace".to_string(),
        Key::Space => "Space".to_string(),
        _ => format!("{:?}", key),
    }
}