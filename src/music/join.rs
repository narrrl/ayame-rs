use std::{sync::Arc, time::Duration};

use poise::serenity_prelude::{ChannelId, Mutex};
use songbird::{Call, Event};

use super::{MusicContext, NotificationHandler, TimeoutHandler};
use crate::Error;

async fn add_events(mtx: &MusicContext, call: Arc<Mutex<Call>>) {
    let mut call = call.lock().await;
    call.remove_all_global_events();

    call.add_global_event(
        Event::Periodic(Duration::from_secs(60), None),
        TimeoutHandler { mtx: mtx.clone() },
    );

    call.add_global_event(
        Event::Periodic(Duration::from_millis(3000), None),
        NotificationHandler { mtx: mtx.clone() },
    );
}

pub async fn join(
    mtx: &MusicContext,
    voice_channel_id: &ChannelId,
) -> Result<Arc<Mutex<Call>>, Error> {
    let (call, success) = mtx.songbird.join(mtx.guild_id.0, voice_channel_id.0).await;

    if let Err(why) = success {
        return Err(Error::from(why));
    }

    add_events(mtx, call.clone()).await;

    Ok(call)
}
