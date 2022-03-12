use mensa_swfr_rs::error::MensaError;
use poise::serenity_prelude::Error as SerenityError;
use songbird::error::{ConnectionError, JoinError};
use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum AyameError {
    #[error("{0}")]
    Input(&'static str),
    #[error("{0}")]
    Failure(&'static str),
    #[error("{:?}", source)]
    Serenity {
        #[from]
        source: SerenityError,
    },
    #[error("{:?}", source)]
    JoinError {
        #[from]
        source: JoinError,
    },
    #[error("{:?}", source)]
    ConnectionError {
        #[from]
        source: ConnectionError,
    },
    #[error("{:?}", source)]
    Mensa {
        #[from]
        source: MensaError,
    },
}

// should be save, because `poise::serenity_prelude::Error` implements it and `&'static str` is
// static anyway. Let's hope for the best lol
unsafe impl Send for AyameError {}
unsafe impl Sync for AyameError {}
