use thiserror::Error;
use crate::ps1;

pub type MipsResult<T> = Result<T, MipsError>;

#[derive(Error, Debug)]
pub enum MipsError {
    #[error("PS1 error: {0}")]
    Ps1Error(#[from] ps1::Ps1Error),
}