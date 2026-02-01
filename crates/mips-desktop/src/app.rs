use std::env;
use std::time::Instant;
use egui::{ColorImage, TextureHandle, TextureOptions};
use tracing::info;
use mips_core::ConsoleManager;
use mips_core::input::{DeviceType, InputConfig};
use crate::audio::AudioManager;
use crate::input::{InputManager, GamepadManager};

pub struct EmulatorApp {
    // Emulator core
    mips: ConsoleManager,

    // Audio
    audio: AudioManager,

    // Input
    input: InputManager,
    gamepad: GamepadManager,
    button_map: std::collections::HashMap<String, mips_core::input::Button>,

    // Rendering
    game_texture: Option<TextureHandle>,

    // UI state
    show_settings: bool,
    show_input_config: bool,
    show_about: bool,
    paused: bool,

    // Settings
    settings: Settings,

    // Performance tracking
    last_frame: Instant,
    fps: f32,
}

#[derive(Default)]
struct Settings {
    vsync: bool,
    bilinear_filter: bool,
    volume: f32,
    fast_boot: bool,
}

impl EmulatorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        info!("Initializing MIPS emulator");

        // Load game
        let sys_dir = env::current_dir().unwrap();
        let mut mips = ConsoleManager::new();
        if let Err(e) = mips.load_game(sys_dir.as_path(), Some("Silent Hill (USA).cue")) {
            tracing::error!("Failed to load game: {}", e);
        }

        // Setup input
        let input = InputManager::new();
        let config = InputConfig::from("assets/config/profile.input.ini".as_ref());
        let button_map = config.bindings();

        // Connect keyboard to port 0
        mips.connect_device(0, DeviceType::Keyboard);

        // Setup audio
        let audio = AudioManager::new().expect("Failed to initialize audio");

        // Setup gamepad
        let gamepad = GamepadManager::new();

        Self {
            mips,
            audio,
            input,
            gamepad,
            button_map,
            game_texture: None,
            show_settings: false,
            show_input_config: false,
            show_about: false,
            paused: false,
            settings: Settings {
                vsync: true,
                bilinear_filter: false,
                volume: 1.0,
                fast_boot: false,
            },
            last_frame: Instant::now(),
            fps: 60.0,
        }
    }

    fn update_emulator(&mut self, ctx: &egui::Context) {
        if self.paused {
            return;
        }

        // Handle input
        let mut button_queue = self.input.poll_input(ctx, &self.button_map);
        self.gamepad.poll_gamepad(&mut button_queue);
        self.mips.handle_inputs(button_queue);
        self.mips.refresh_devices();

        // Update emulator
        self.mips.update();

        // Handle audio
        let audio_samples = self.mips.get_audio_samples();
        self.audio.queue_samples(audio_samples);
        self.mips.clear_audio_samples();

        // Update FPS counter
        let now = Instant::now();
        let delta = now.duration_since(self.last_frame).as_secs_f32();
        if delta > 0.0 {
            self.fps = 0.9 * self.fps + 0.1 * (1.0 / delta);
        }
        self.last_frame = now;
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

                // FPS counter on the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("FPS: {:.1}", self.fps));
                });
            });
        });
    }

    fn render_game(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(frame) = self.mips.get_frame() {
                // Convert frame to egui ColorImage
                let image = ColorImage::from_rgba_premultiplied(
                    [frame.width as usize, frame.height as usize],
                    bytemuck::cast_slice(&frame.pixels),
                );

                // Update texture
                let texture_options = if self.settings.bilinear_filter {
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
                    let game_aspect = frame.width as f32 / frame.height as f32;
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
                ui.checkbox(&mut self.settings.vsync, "VSync");
                ui.checkbox(&mut self.settings.bilinear_filter, "Bilinear Filtering");

                ui.separator();
                ui.heading("Audio");
                ui.add(
                    egui::Slider::new(&mut self.settings.volume, 0.0..=1.0)
                        .text("Volume")
                );
                self.audio.set_volume(self.settings.volume);

                ui.separator();
                ui.heading("System");
                ui.checkbox(&mut self.settings.fast_boot, "Skip BIOS");

                ui.separator();
                if ui.button("Close").clicked() {
                    self.show_settings = false;
                }
            });
        self.show_settings = show_settings;
    }

    fn render_input_config(&mut self, ctx: &egui::Context) {
        if !self.show_input_config {
            return;
        }

        egui::Window::new("Input Configuration")
            .open(&mut self.show_input_config)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Keyboard controls");
                ui.separator();

                egui::Grid::new("input_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Action");
                        ui.label("Key");
                        ui.end_row();

                        ui.label("D-Pad Up:");
                        ui.label("↑");
                        ui.end_row();

                        ui.label("D-Pad Down:");
                        ui.label("↓");
                        ui.end_row();

                        ui.label("D-Pad Left:");
                        ui.label("←");
                        ui.end_row();

                        ui.label("D-Pad Right:");
                        ui.label("→");
                        ui.end_row();

                        ui.label("Cross (X):");
                        ui.label("Z");
                        ui.end_row();

                        ui.label("Circle (O):");
                        ui.label("X");
                        ui.end_row();

                        ui.label("Square:");
                        ui.label("A");
                        ui.end_row();

                        ui.label("Triangle:");
                        ui.label("S");
                        ui.end_row();
                    });

                ui.separator();
                ui.label("Note: Input rebinding coming soon!");
            });
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
                ui.separator();
                ui.hyperlink_to("GitHub", "https://github.com/yourusername/mips");
            });
    }
}

impl eframe::App for EmulatorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update emulator state
        self.update_emulator(ctx);

        // Render UI
        self.render_menu_bar(ctx);
        self.render_game(ctx);
        self.render_settings(ctx);
        self.render_input_config(ctx);
        self.render_about(ctx);

        // Request continuous repaints for smooth emulation
        ctx.request_repaint();
    }
}