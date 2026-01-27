use thiserror::Error;
use mips_core::MipsError;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Window failed to build: {0}")]
    WindowBuildFailure(String),
    #[error("SDL error: {0}")]
    SdlError(#[from] sdl3::Error),
    #[error("Mips error: {0}")]
    MipsError(#[from] MipsError),
}