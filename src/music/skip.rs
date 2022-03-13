use poise::serenity_prelude::GuildId;

use crate::{Context, Error};

use crate::error::*;

pub async fn skip(ctx: &Context<'_>, guild_id: &GuildId) -> Result<(), Error> {
    let call = super::get_poise(&ctx)
        .await?
        .get(guild_id.0)
        .ok_or_else(|| Error::Input(NOTHING_PLAYING))?;

    let call = call.lock().await;

    let queue = call.queue();

    queue.skip()?;
    Ok(())
}
