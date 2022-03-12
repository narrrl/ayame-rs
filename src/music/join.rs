use std::{sync::Arc, time::Duration};

use poise::serenity_prelude::{ChannelId, Mutex, Result, SerenityError};
use songbird::{Call, Event};

use super::{MusicContext, TimeoutHandler};
use crate::utils::check_result;
use crate::Context;
use once_cell::sync::Lazy;

static IS_CONNECTING: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

async fn add_events(mtx: &MusicContext, call: Arc<Mutex<Call>>) {
    let mut call = call.lock().await;
    call.remove_all_global_events();

    call.add_global_event(
        Event::Periodic(Duration::from_secs(60), None),
        TimeoutHandler { mtx: mtx.clone() },
    );
}

pub async fn join(
    ctx: &Context<'_>,
    mtx: &MusicContext,
    voice_channel_id: &ChannelId,
) -> Result<Arc<Mutex<Call>>> {
    let songbird = match super::get_serenity(&mtx.ctx).await {
        Some(songbird) => songbird,
        None => {
            return Err(SerenityError::Other("error getting songbird"));
        }
    };

    let guard = IS_CONNECTING.lock().await;

    let (call, success) = songbird.join(mtx.guild_id.0, voice_channel_id.0).await;

    drop(guard);

    let _ = success.map_err(|e| async move {
        check_result(
            ctx.send(|m| m.content(format!("error joining channel: {}", e)))
                .await,
        );
    });

    add_events(mtx, call.clone()).await;

    Ok(call)
}
