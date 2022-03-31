use std::sync::Arc;

use poise::serenity_prelude::Mutex;
use songbird::input::{Input, Restartable};
use songbird::tracks::TrackHandle;
use songbird::Call;
use tokio::join;

use crate::error::*;
use crate::{Context, Error};

pub async fn play(call: &Arc<Mutex<Call>>, source: Input) -> Result<TrackHandle, Error> {
    let mut call_lock = call.lock().await;
    let track_handle = call_lock.enqueue_source(source);

    Ok(track_handle)
}

pub async fn register_and_play(ctx: Context<'_>, song: String) -> Result<TrackHandle, Error> {
    let guild = ctx.guild().ok_or_else(|| Error::Input(NOT_IN_GUILD))?;
    let channel_id = match guild.voice_states.get(&ctx.author().id) {
        Some(state) => match state.channel_id {
            Some(id) => id,
            None => return Err(Error::Input(NOT_IN_VOICE)),
        },
        None => return Err(Error::Input(NOT_IN_VOICE)),
    };
    let source_future = Restartable::ytdl(song, true);
    let call_future = super::get_call_else_join(&ctx, &guild.id, &channel_id);

    let (source, call) = join!(source_future, call_future);
    let (call, source) = (call?, source?);

    let track = super::play::play(&call, source.into()).await?;

    let uuid = track.uuid();

    let user_id = ctx.author().id;

    let mut requests = ctx.data().song_queues.lock().await;
    requests.insert(uuid, user_id);
    drop(requests);

    Ok(track)
}
