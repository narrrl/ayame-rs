use songbird::id::GuildId;

use crate::Error;

use super::MusicContext;

pub async fn leave(mtx: &MusicContext) -> Result<(), Error> {
    let songbird = super::get_serenity(&mtx.ctx).await?;
    songbird.remove(GuildId::from(mtx.guild_id.0)).await?;
    Ok(())
}
