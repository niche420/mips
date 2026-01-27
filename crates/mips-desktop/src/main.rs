use crate::app::App;

mod error;
mod audio;
mod input;
mod app;
mod wnd;
mod evt;

fn main() -> anyhow::Result<()> {
    let mut app = App::new()?;
    app.run();

    Ok(())
}