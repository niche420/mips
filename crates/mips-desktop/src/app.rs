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
    cached_frame: Option<CachedFrame>,

    // UI state
    show_settings: bool,
    show_input_config: bool,
    show_about: bool,
    paused: bool,

    // Settings
    settings: Settings,

    // Performance tracking
    last_emulator_update: Instant,
    frame_debt: f64, // Track fractional frames
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
            cached_frame: None,
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
        // Handle input
        let mut button_queue = self.input.poll_input(ctx, &self.button_map);
        self.gamepad.poll_gamepad(&mut button_queue);
        self.mips.handle_inputs(button_queue);
        self.mips.refresh_devices();

        // Update emulator - ONE frame
        self.mips.update();

        // Handle audio
        let audio_samples = self.mips.get_audio_samples();
        self.audio.queue_samples(audio_samples);
        self.mips.clear_audio_samples();

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
                    ui.separator();

                    // VSync toggle button
                    let vsync_text = if self.settings.vsync { "VSync: ON" } else { "VSync: OFF" };
                    if ui.button(vsync_text).clicked() {
                        self.settings.vsync = !self.settings.vsync;
                    }
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
        // Update emulator (adaptive timing)
        self.update_emulator(ctx);

        // Render UI
        self.render_menu_bar(ctx);
        self.render_game(ctx);
        self.render_settings(ctx);
        self.render_input_config(ctx);
        self.render_about(ctx);

        // Always request repaint to keep emulator running smoothly
        ctx.request_repaint();
    }
}