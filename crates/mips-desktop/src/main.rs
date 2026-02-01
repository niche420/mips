use tracing::info;
use crate::app::App;

mod error;
mod audio;
mod input;
mod app;
mod wnd;
mod evt;
mod ui;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    info!("Begin log");

    let mut app = App::new()?;
    app.run();

    Ok(())
}