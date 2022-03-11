use poise::serenity_prelude::Result;
use tracing::error;

use super::MusicContext;

pub async fn join<'a, U, E>(mtx: &MusicContext<'a, U, E>) -> Result<()>
where
    U: Send + Sync,
    E: Send + Sync,
{
    let songbird = super::get(&mtx.ctx)
        .await
        .ok_or_else(|| error!("couldn't get songbird"));

    Ok(())
}
