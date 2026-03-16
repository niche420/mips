mod error;
mod audio;
mod input;
mod app;
mod wnd;
mod evt;
mod ui;
mod config;

use anyhow::Result;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Configure the native window
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("MIPS - PlayStation Emulator"),
        ..Default::default()
    };

    // Run the app
    eframe::run_native(
        "MIPS",
        native_options,
        Box::new(|cc| Ok(Box::new(app::EmulatorApp::new(cc)))),
    ).map_err(|e| anyhow::anyhow!("eframe error: {}", e))
}