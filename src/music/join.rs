use poise::serenity_prelude::Result;
use songbird::id::GuildId;
use tracing::error;

use super::MusicContext;

pub async fn join(mtx: &MusicContext) -> Result<()> {
    let songbird = super::get(&mtx.ctx)
        .await
        .ok_or_else(|| error!("couldn't get songbird"))?;

    Ok(())
}
