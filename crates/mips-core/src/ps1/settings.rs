use crate::ps1::settings::graphics::GraphicsSettings;

pub mod graphics;
mod cd;

#[derive(Default)]
pub struct Ps1Settings {
    graphics: GraphicsSettings,
}