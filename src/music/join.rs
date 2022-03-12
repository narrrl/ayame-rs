use std::{sync::Arc, time::Duration};

use poise::serenity_prelude::{ChannelId, Mutex, Result};
use songbird::{Call, Event};
use tracing::error;

use super::{MusicContext, TimeoutHandler};
use crate::utils::check_result;
use once_cell::sync::Lazy;

static IS_CONNECTING: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

async fn add_events<'a>(mtx: &MusicContext<'a>, call: Arc<Mutex<Call>>) {
    let mut call = call.lock().await;
    call.remove_all_global_events();

    call.add_global_event(
        Event::Periodic(Duration::from_secs(60), None),
        TimeoutHandler { mtx: mtx.clone() },
    );
}

pub async fn join(mtx: &MusicContext<'_>, voice_channel_id: &ChannelId) -> Result<()> {
    let songbird = match super::get(&mtx.ctx).await {
        Some(songbird) => songbird,
        None => {
            error!("error getting songbird");
            return Ok(());
        }
    };

    let guard = IS_CONNECTING.lock().await;

    let (call, success) = songbird.join(mtx.guild_id.0, voice_channel_id.0).await;

    drop(guard);

    let _ = success.map_err(|e| async move {
        check_result(
            mtx.send(|m| m.content(format!("error joining channel: {}", e)))
                .await,
        );
    });

    add_events(mtx, call).await;

    Ok(())
}
