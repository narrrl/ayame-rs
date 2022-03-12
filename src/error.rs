use thiserror::Error;

#[derive(Error, Clone, Debug)]
pub enum AyameError {
    #[error("failed to execute: {0}")]
    Failure(String),
}

impl From<&str> for AyameError {
    fn from(fr: &str) -> AyameError {
        AyameError::Failure(fr.to_string())
    }
}
