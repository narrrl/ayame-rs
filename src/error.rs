use mensa_swfr_rs::error::MensaError;
use poise::serenity_prelude::{self as serenity, Color, Error as SerenityError};
use regex::Error as RegexError;
use reqwest::Error as ReqwestError;
use songbird::error::{ConnectionError, JoinError};
use songbird::input::error::Error as SongbirdError;
use songbird::tracks::TrackError;
use sqlx::Error as SQLError;
use std::io::Error as IOError;
use thiserror::Error;

use crate::{utils::check_result, Context};

pub const CHANNEL_ALREADY_BOUND: &'static str = "channel was already bound";
pub const ONLY_IN_BOT_CHANNEL: &'static str = "command can only be executed in bot channel";
pub const NO_BOT_CHANNEL: &'static str = "there is no bot channel, ask an admin to fix this";
pub const WRONG_CHANNEL_TO_BIND: &'static str = "you can only bind text channel";
pub const NOT_IN_GUILD: &'static str = "only in guilds";
pub const NOT_IN_VOICE: &'static str = "not in a voice channel";
pub const NO_SEARCH_RESULTS: &'static str = "nothing found";
pub const NOTHING_PLAYING: &'static str = "nothing playing";
pub const FAILD_TO_GET_SONGBIRD: &'static str = "couldn't get songbird";
pub const UNKNOWN_WEEKDAY: &'static str = "unknown weekday";
pub const NO_MENSA_KEY: &'static str = "no mensa key provided";
pub const UNKNOWN_RESPONSE: &'static str = "got unknown response";
pub const EVENT_CANCELED: &'static str = "the event was canceled";
pub const COULDNT_GET_MSG: &'static str = "couldn't get message";
// pub const EMPTY_MENU: &'static str = "can't initialize empty menu";

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
    SongbirdError {
        #[from]
        source: SongbirdError,
    },
    #[error("{:?}", source)]
    ReqwestError {
        #[from]
        source: ReqwestError,
    },
    #[error("{:?}", source)]
    TrackError {
        #[from]
        source: TrackError,
    },
    #[error("{:?}", source)]
    IOError {
        #[from]
        source: IOError,
    },
    #[error("{:?}", source)]
    MensaError {
        #[from]
        source: MensaError,
    },
    #[error("{:?}", source)]
    RegexError {
        #[from]
        source: RegexError,
    },
    #[error("{:?}", source)]
    DatabaseError {
        #[from]
        source: SQLError,
    },
}

// should be save, because `poise::serenity_prelude::Error` implements it and `&'static str` is
// static anyway. Let's hope for the best lol
unsafe impl Send for AyameError {}
unsafe impl Sync for AyameError {}

impl AyameError {
    pub async fn send_error(&self, ctx: &Context<'_>) {
        check_result(
            ctx.send(|m| {
                m.embed(|em| {
                    em.title("Error!")
                        .color(Color::RED)
                        .description(&self.to_string())
                })
                .ephemeral(true)
            })
            .await,
        )
    }

    pub fn set_error_embed<'a>(
        &self,
        e: &'a mut serenity::CreateEmbed,
    ) -> &'a mut serenity::CreateEmbed {
        e.title("Error!")
            .color(Color::RED)
            .description(&self.to_string())
    }
}
