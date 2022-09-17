use crate::{error::Error as AYError, Context, Error};
use mensa_swfr_rs as mensa_fr;
use poise::serenity_prelude as serenity;

/// shows all mensa plans for freiburg
#[poise::command(slash_command, track_edits, category = "University Freiburg")]
pub async fn mensa(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    Err(Box::new(AYError::InvalidInput("not implemented")))
}
