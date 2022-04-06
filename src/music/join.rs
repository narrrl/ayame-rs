use std::{sync::Arc, time::Duration};

use poise::serenity_prelude::{self as serenity, ChannelId, Mutex};
use songbird::{Call, Event};

use super::{MusicContext, NotificationHandler};
use crate::Error;

async fn add_events(mtx: MusicContext, call: Arc<Mutex<Call>>) {
    let mut call = call.lock().await;
    call.remove_all_global_events();

    call.add_global_event(
        Event::Delayed(Duration::from_millis(1000)),
        NotificationHandler { mtx: mtx.clone() },
    );
    call.add_global_event(
        Event::Periodic(Duration::from_millis(5000), None),
        NotificationHandler { mtx },
    );
}

pub async fn join(
    ctx: &crate::Context<'_>,
    guild_id: &serenity::GuildId,
    voice_channel_id: &ChannelId,
) -> Result<Arc<Mutex<Call>>, Error> {
    let res = join_serenity(ctx.discord(), ctx.data(), guild_id, voice_channel_id).await;
    if let Ok(_) = res {
        super::now_playing::now_playing(&ctx, &guild_id).await?;
    }
    res
}

pub async fn join_serenity(
    ctx: &serenity::Context,
    data: &crate::Data,
    guild_id: &serenity::GuildId,
    voice_channel_id: &ChannelId,
) -> Result<Arc<Mutex<Call>>, Error> {
    let mtx = MusicContext {
        songbird: super::get_serenity(&ctx).await?,
        guild_id: *guild_id,
        data: data.clone(),
        http: ctx.http.clone(),
        cache: ctx.cache.clone(),
    };

    let (call, success) = mtx.songbird.join(mtx.guild_id.0, voice_channel_id.0).await;

    if let Err(why) = success {
        return Err(Error::from(why));
    }

    add_events(mtx, call.clone()).await;

    Ok(call)
}
