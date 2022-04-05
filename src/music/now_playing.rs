use poise::serenity_prelude as serenity;

use super::MusicContext;

pub async fn now_playing(
    mtx: MusicContext,
    channel_id: serenity::ChannelId,
) -> Result<(), crate::Error> {
    tokio::spawn(async move {
        while let Some(call) = mtx.songbird.get(mtx.guild_id) {
            let call_lock = call.lock().await;
            let queue = call_lock.queue();
            let embed = match queue.current() {
                Some(current) => {
                    let user_map = mtx.data.song_queues.lock().await;
                    let user = match user_map.get(&current.uuid()) {
                        Some(id) => id.to_user_cached(&mtx.cache).await,
                        None => None,
                    };

                    super::embed_upcoming(&queue, super::embed_song(&current, user).await)
                }
                None => {
                    let mut e = serenity::CreateEmbed::default();
                    e.title("Nothing is playing!");
                    e
                }
            };
            let mut msgs = mtx.data.playing_messages.lock().await;
            if let Some(msg_id) = msgs.get(&mtx.guild_id) {
                match mtx
                    .http
                    .get_message(*channel_id.as_u64(), *msg_id.as_u64())
                    .await
                {
                    Ok(mut msg) => {
                        if let Err(_) = msg.edit(&mtx.http, |edit| edit.set_embed(embed)).await {
                            msgs.remove(&mtx.guild_id);
                        }
                    }
                    Err(_) => {
                        msgs.remove(&mtx.guild_id);
                    }
                };
            } else {
            }
        }
    });
    Ok(())
}
