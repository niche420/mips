use std::env;
use std::time::Instant;
use egui::{ColorImage, TextureHandle, TextureOptions, Key};
use tracing::info;
use mips_core::ConsoleManager;
use mips_core::input::{DeviceType, Button};
use crate::audio::AudioManager;
use crate::input::{InputManager, GamepadManager};
use crate::config::{ConfigManager, button_display_name, key_display_name};
use gilrs::Button as GilrsButton;

pub struct EmulatorApp {
    // Emulator core
    mips: ConsoleManager,

    // Configuration
    config: ConfigManager,

    // Audio
    audio: AudioManager,

    // Input
    input: InputManager,
    gamepad: GamepadManager,

    // Rendering
    game_texture: Option<TextureHandle>,
    cached_frame: Option<CachedFrame>,

    // UI state
    show_settings: bool,
    show_input_config: bool,
    show_about: bool,
    paused: bool,

    // Input config state
    input_config_tab: InputConfigTab,
    waiting_for_key: Option<Button>,
    waiting_for_gamepad_button: Option<Button>,

    // Performance tracking
    last_emulator_update: Instant,
    frame_debt: f64,
    emulation_fps: f32,
    emulation_frame_count: u32,
    emulation_fps_timer: Instant,
}

#[derive(Clone)]
struct CachedFrame {
    rgba_pixels: Vec<u8>,
    width: usize,
    height: usize,
}

#[derive(PartialEq)]
enum InputConfigTab {
    Keyboard,
    Gamepad,
}

impl EmulatorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        info!("Initializing MIPS emulator");

        // Load configuration
        let config = ConfigManager::new().expect("Failed to load configuration");

        // Load game
        let sys_dir = env::current_dir().unwrap();
        let mut mips = ConsoleManager::new();
        if let Err(e) = mips.load_game(sys_dir.as_path(), Some("Silent Hill (USA).cue")) {
            tracing::error!("Failed to load game: {}", e);
        }

        // Setup input
        let input = InputManager::new();
        let gamepad = GamepadManager::new();

        // Connect keyboard to port 0
        mips.connect_device(0, DeviceType::Keyboard);

        // Setup audio
        let mut audio = AudioManager::new().expect("Failed to initialize audio");
        audio.set_volume(config.settings.audio.volume);

        Self {
            mips,
            config,
            audio,
            input,
            gamepad,
            game_texture: None,
            cached_frame: None,
            show_settings: false,
            show_input_config: false,
            show_about: false,
            paused: false,
            input_config_tab: InputConfigTab::Keyboard,
            waiting_for_key: None,
            waiting_for_gamepad_button: None,
            last_emulator_update: Instant::now(),
            frame_debt: 0.0,
            emulation_fps: 60.0,
            emulation_frame_count: 0,
            emulation_fps_timer: Instant::now(),
        }
    }

    fn update_emulator(&mut self, ctx: &egui::Context) {
        if self.paused {
            return;
        }

        const TARGET_FPS: f64 = 60.0;
        const FRAME_TIME: f64 = 1.0 / TARGET_FPS;

        let now = Instant::now();
        let delta = now.duration_since(self.last_emulator_update).as_secs_f64();
        self.last_emulator_update = now;

        // Accumulate frame debt
        self.frame_debt += delta / FRAME_TIME;

        // Run emulator frames to pay off debt
        // Limit to max 2 frames per update to prevent audio issues
        let frames_to_run = self.frame_debt.floor().min(2.0) as u32;

        for _ in 0..frames_to_run {
            self.run_emulator_frame(ctx);
            self.frame_debt -= 1.0;

            // Count for FPS display
            self.emulation_frame_count += 1;
        }

        // Update FPS counter
        if self.emulation_fps_timer.elapsed() >= std::time::Duration::from_secs(1) {
            self.emulation_fps = self.emulation_frame_count as f32;
            self.emulation_frame_count = 0;
            self.emulation_fps_timer = Instant::now();
        }
    }

    fn run_emulator_frame(&mut self, ctx: &egui::Context) {
        // Handle audio
        if self.config.settings.audio.enabled {
            let audio_samples = self.mips.get_audio_samples();
            self.audio.enqueue(audio_samples);
        }
        self.mips.clear_audio_samples();

        // Handle input (only if not configuring)
        if !self.show_input_config {
            let mut button_queue = self.input.poll_input(ctx, &self.config.keyboard_bindings.bindings);
            self.gamepad.poll_gamepad(&mut button_queue, &self.config.gamepad_bindings.bindings);
            self.mips.handle_inputs(button_queue);
            self.mips.refresh_devices();
        }

        // Update emulator - ONE frame
        self.mips.update();

        // Cache the frame if we got a new one
        if let Some(frame) = self.mips.get_frame() {
            // Convert XRGB (0xAARRGGBB) to RGBA bytes
            let rgba_pixels: Vec<u8> = frame.pixels.iter()
                .flat_map(|&pixel| {
                    let r = ((pixel >> 16) & 0xFF) as u8;
                    let g = ((pixel >> 8) & 0xFF) as u8;
                    let b = (pixel & 0xFF) as u8;
                    let a = 255u8;
                    [r, g, b, a]
                })
                .collect();

            self.cached_frame = Some(CachedFrame {
                rgba_pixels,
                width: frame.width as usize,
                height: frame.height as usize,
            });
        }
    }

    fn render_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open ROM...").clicked() {
                        // TODO: File dialog
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        // Save settings before exit
                        let _ = self.config.save_settings();
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Emulation", |ui| {
                    let pause_text = if self.paused { "Resume" } else { "Pause" };
                    if ui.button(pause_text).clicked() {
                        self.paused = !self.paused;
                        ui.close_menu();
                    }
                    if ui.button("Reset").clicked() {
                        // TODO: Reset emulator
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Save State").clicked() {
                        // TODO: Save state
                        ui.close_menu();
                    }
                    if ui.button("Load State").clicked() {
                        // TODO: Load state
                        ui.close_menu();
                    }
                });

                ui.menu_button("Options", |ui| {
                    if ui.button("Settings...").clicked() {
                        self.show_settings = true;
                        ui.close_menu();
                    }
                    if ui.button("Input Configuration...").clicked() {
                        self.show_input_config = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        self.show_about = true;
                        ui.close_menu();
                    }
                });

                // FPS counter and VSync toggle on the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("FPS: {:.0}", self.emulation_fps));
                });
            });
        });
    }

    fn render_game(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Use cached frame to prevent flickering
            if let Some(cached) = &self.cached_frame {
                // Create ColorImage from cached RGBA data
                let image = ColorImage::from_rgba_unmultiplied(
                    [cached.width, cached.height],
                    &cached.rgba_pixels,
                );

                // Update texture
                let texture_options = if self.config.settings.video.bilinear_filter {
                    TextureOptions::LINEAR
                } else {
                    TextureOptions::NEAREST
                };

                self.game_texture = Some(ctx.load_texture(
                    "game_frame",
                    image,
                    texture_options,
                ));

                if let Some(texture) = &self.game_texture {
                    // Calculate size to maintain aspect ratio
                    let available_size = ui.available_size();
                    let game_aspect = cached.width as f32 / cached.height as f32;
                    let available_aspect = available_size.x / available_size.y;

                    let display_size = if available_aspect > game_aspect {
                        egui::vec2(available_size.y * game_aspect, available_size.y)
                    } else {
                        egui::vec2(available_size.x, available_size.x / game_aspect)
                    };

                    // Center the image
                    ui.centered_and_justified(|ui| {
                        ui.image(egui::load::SizedTexture::new(
                            texture.id(),
                            display_size,
                        ));
                    });
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.heading("No game loaded");
                    ui.label("Select File > Open ROM to load a game");
                });
            }
        });
    }

    fn render_settings(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }

        let mut show_settings = self.show_settings;
        egui::Window::new("Settings")
            .open(&mut show_settings)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Video");

                let mut vsync_changed = false;
                if ui.checkbox(&mut self.config.settings.video.vsync, "VSync").changed() {
                    vsync_changed = true;
                }

                ui.checkbox(&mut self.config.settings.video.bilinear_filter, "Bilinear Filtering");

                ui.separator();
                ui.heading("Audio");

                ui.checkbox(&mut self.config.settings.audio.enabled, "Enable Audio");

                if ui.add(
                    egui::Slider::new(&mut self.config.settings.audio.volume, 0.0..=1.0)
                        .text("Volume")
                ).changed() {
                    self.audio.set_volume(self.config.settings.audio.volume);
                }

                ui.separator();
                ui.heading("System");
                ui.checkbox(&mut self.config.settings.system.fast_boot, "Skip BIOS");
                ui.checkbox(&mut self.config.settings.system.auto_save_state, "Auto-save state on exit");

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        if let Err(e) = self.config.save_settings() {
                            tracing::error!("Failed to save settings: {}", e);
                        }
                        self.show_settings = false;
                    }

                    if ui.button("Reset to Defaults").clicked() {
                        if let Err(e) = self.config.reset_to_defaults() {
                            tracing::error!("Failed to reset settings: {}", e);
                        }
                        self.audio.set_volume(self.config.settings.audio.volume);
                    }

                    if ui.button("Cancel").clicked() {
                        // Reload settings from disk
                        if let Ok(new_config) = ConfigManager::new() {
                            self.config = new_config;
                            self.audio.set_volume(self.config.settings.audio.volume);
                        }
                        self.show_settings = false;
                    }
                });
            });
        self.show_settings = show_settings;
    }

    fn render_input_config(&mut self, ctx: &egui::Context) {
        if !self.show_input_config {
            return;
        }

        let mut show_input_config = self.show_input_config;

        egui::Window::new("Input Configuration")
            .open(&mut show_input_config)
            .resizable(false)
            .default_width(500.0)
            .show(ctx, |ui| {
                // Tab selection
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.input_config_tab, InputConfigTab::Keyboard, "Keyboard");
                    ui.selectable_value(&mut self.input_config_tab, InputConfigTab::Gamepad, "Gamepad");
                });

                ui.separator();

                match self.input_config_tab {
                    InputConfigTab::Keyboard => self.render_keyboard_config(ui, ctx),
                    InputConfigTab::Gamepad => self.render_gamepad_config(ui, ctx),
                }

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        if let Err(e) = self.config.save_keyboard_bindings() {
                            tracing::error!("Failed to save keyboard bindings: {}", e);
                        }
                        if let Err(e) = self.config.save_gamepad_bindings() {
                            tracing::error!("Failed to save gamepad bindings: {}", e);
                        }
                        self.show_input_config = false;
                        self.waiting_for_key = None;
                        self.waiting_for_gamepad_button = None;
                    }

                    if ui.button("Reset to Defaults").clicked() {
                        if let Err(e) = self.config.reset_to_defaults() {
                            tracing::error!("Failed to reset bindings: {}", e);
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        // Reload bindings from disk
                        if let Ok(new_config) = ConfigManager::new() {
                            self.config.keyboard_bindings = new_config.keyboard_bindings;
                            self.config.gamepad_bindings = new_config.gamepad_bindings;
                        }
                        self.show_input_config = false;
                        self.waiting_for_key = None;
                        self.waiting_for_gamepad_button = None;
                    }
                });
            });

        self.show_input_config = show_input_config;
    }

    fn render_keyboard_config(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if let Some(waiting_button) = self.waiting_for_key {
            ui.label(format!("Press a key for {}...", button_display_name(&waiting_button)));
            ui.label("(Press ESC to cancel)");

            // Check for key press
            ctx.input(|i| {
                if i.key_pressed(Key::Escape) {
                    self.waiting_for_key = None;
                    return;
                }

                // Check for any key press
                for key in [
                    Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G, Key::H,
                    Key::I, Key::J, Key::K, Key::L, Key::M, Key::N, Key::O, Key::P,
                    Key::Q, Key::R, Key::S, Key::T, Key::U, Key::V, Key::W, Key::X,
                    Key::Y, Key::Z,
                    Key::ArrowUp, Key::ArrowDown, Key::ArrowLeft, Key::ArrowRight,
                    Key::Enter, Key::Space, Key::Backspace,
                ] {
                    if i.key_pressed(key) {
                        // Remove old binding for this key
                        self.config.keyboard_bindings.bindings.retain(|k, _| k != &key);
                        // Add new binding
                        self.config.keyboard_bindings.bindings.insert(key, waiting_button);
                        self.waiting_for_key = None;
                        return;
                    }
                }
            });
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("keyboard_grid")
                    .num_columns(3)
                    .spacing([10.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Button");
                        ui.label("Key");
                        ui.label("");
                        ui.end_row();

                        // Define button order
                        let buttons = [
                            Button::DUp, Button::DDown, Button::DLeft, Button::DRight,
                            Button::Cross, Button::Circle, Button::Square, Button::Triangle,
                            Button::L1, Button::R1, Button::L2, Button::R2,
                            Button::Start, Button::Select,
                        ];

                        for button in buttons {
                            ui.label(button_display_name(&button));

                            // Find current key binding
                            let current_key = self.config.keyboard_bindings.bindings
                                .iter()
                                .find(|(_, b)| **b == button)
                                .map(|(k, _)| *k);

                            let key_text = current_key
                                .map(|k| key_display_name(&k))
                                .unwrap_or_else(|| "Unbound".to_string());

                            ui.label(key_text);

                            if ui.button("Change").clicked() {
                                self.waiting_for_key = Some(button);
                            }

                            ui.end_row();
                        }
                    });
            });
        }
    }

    fn render_gamepad_config(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if let Some(waiting_button) = self.waiting_for_gamepad_button {
            ui.label(format!("Press a gamepad button for {}...", button_display_name(&waiting_button)));
            ui.label("(Press any key to cancel)");

            // Check for gamepad button press
            if let Some(gilrs) = &mut self.gamepad.gilrs {
                while let Some(event) = gilrs.next_event() {
                    if let gilrs::EventType::ButtonPressed(gilrs_button, _) = event.event {
                        // Remove old binding for this button
                        self.config.gamepad_bindings.bindings.retain(|b, _| b != &gilrs_button);
                        // Add new binding
                        self.config.gamepad_bindings.bindings.insert(gilrs_button, waiting_button);
                        self.waiting_for_gamepad_button = None;
                        return;
                    }
                }
            }

            // Check for cancel
            ctx.input(|i| {
                if !i.keys_down.is_empty() {
                    self.waiting_for_gamepad_button = None;
                }
            });
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("gamepad_grid")
                    .num_columns(3)
                    .spacing([10.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("PS1 Button");
                        ui.label("Gamepad Button");
                        ui.label("");
                        ui.end_row();

                        let buttons = [
                            Button::DUp, Button::DDown, Button::DLeft, Button::DRight,
                            Button::Cross, Button::Circle, Button::Square, Button::Triangle,
                            Button::L1, Button::R1, Button::L2, Button::R2,
                            Button::Start, Button::Select,
                        ];

                        for button in buttons {
                            ui.label(button_display_name(&button));

                            // Find current gamepad binding
                            let current_gilrs = self.config.gamepad_bindings.bindings
                                .iter()
                                .find(|(_, b)| **b == button)
                                .map(|(g, _)| *g);

                            let gilrs_text = current_gilrs
                                .map(|g| format!("{:?}", g))
                                .unwrap_or_else(|| "Unbound".to_string());

                            ui.label(gilrs_text);

                            if ui.button("Change").clicked() {
                                self.waiting_for_gamepad_button = Some(button);
                            }

                            ui.end_row();
                        }
                    });
            });
        }
    }

    fn render_about(&mut self, ctx: &egui::Context) {
        if !self.show_about {
            return;
        }

        egui::Window::new("About")
            .open(&mut self.show_about)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("MIPS PlayStation Emulator");
                ui.separator();
                ui.label("A PlayStation 1 emulator written in Rust");
                ui.label("Using egui for UI and cpal for audio");
                ui.separator();
                ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                ui.separator();
                ui.hyperlink_to("GitHub", "https://github.com/yourusername/mips");
            });
    }
}

impl eframe::App for EmulatorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update emulator (adaptive timing)
        self.update_emulator(ctx);

        // Render UI
        self.render_menu_bar(ctx);
        self.render_game(ctx);
        self.render_settings(ctx);
        self.render_input_config(ctx);
        self.render_about(ctx);

        // Request repaint based on vsync setting
        if self.config.settings.video.vsync {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(1.0/60.0));
        } else {
            ctx.request_repaint();
        }
    }
}