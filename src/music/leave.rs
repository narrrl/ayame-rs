use std::sync::Arc;

use poise::serenity_prelude::GuildId;
use songbird::Songbird;

use crate::Error;

pub async fn leave(songbird: &Arc<Songbird>, guild_id: GuildId) -> Result<(), Error> {
    songbird.remove(GuildId::from(guild_id.0)).await?;
    Ok(())
}
