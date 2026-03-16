use thiserror::Error;
use crate::ps1;

pub type MipsResult<T> = Result<T, MipsError>;

#[derive(Error, Debug)]
pub enum MipsError {
    #[error("PS1 error: {0}")]
    Ps1Error(#[from] ps1::Ps1Error),

    #[error("Serde JSON error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Flexbuffers serialization error: {0}")]
    Flexbuffers(#[from] flexbuffers::SerializationError),

    #[error("Invalid state: {0}")]
    InvalidState(String),
}