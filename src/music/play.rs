use std::sync::Arc;

use poise::serenity_prelude::Mutex;
use songbird::input::Input;
use songbird::tracks::TrackHandle;
use songbird::Call;

use crate::Error;

pub async fn play(call: &Arc<Mutex<Call>>, source: Input) -> Result<TrackHandle, Error> {
    let mut call_lock = call.lock().await;
    let track_handle = call_lock.enqueue_source(source);

    Ok(track_handle)
}
