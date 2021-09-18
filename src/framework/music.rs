use humantime::format_duration;
use serenity::{
    async_trait,
    builder::CreateMessage,
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    http::Http,
    model::{channel::Message, misc::Mentionable, prelude::ChannelId},
    prelude::Mutex,
    Result as SerenityResult,
};
use songbird::{
    driver::Bitrate,
    input::{self, restartable::Restartable, Metadata},
    tracks::PlayMode,
    Call, Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent,
};
use std::{sync::Arc, time::Duration};

use tracing::{error, info};

pub const DEFAULT_BITRATE: i32 = 128_000;

pub async fn join<'a>(ctx: &Context, msg: &Message) -> CreateMessage<'a> {
    let mut m = CreateMessage::default();
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            m.content("Not in a voice channel");
            return m;
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let (handle_lock, success) = manager.join(guild_id, connect_to).await;

    if let Ok(_channel) = success {
        let chan_id = msg.channel_id;

        let send_http = ctx.http.clone();

        let mut handle = handle_lock.lock().await;

        let handle_lock = manager.get(guild_id).unwrap();

        let bitrate = {
            if let Some(ch) = handle.current_channel() {
                match ctx.http.get_channel(ch.0).await {
                    Ok(ch) => match ch
                        .guild()
                        .expect(
                            "How did you manage to let the bot join anything but a voice channel?",
                        )
                        .bitrate
                    {
                        Some(bitrate) => bitrate as i32,
                        None => DEFAULT_BITRATE,
                    },
                    Err(_) => DEFAULT_BITRATE,
                }
            } else {
                DEFAULT_BITRATE
            }
        };

        handle.set_bitrate(Bitrate::BitsPerSecond(bitrate.clone()));
        info!("setting bitrate {} for guild {}", bitrate, guild_id);

        handle.add_global_event(
            Event::Track(TrackEvent::End),
            TrackEndNotifier {
                chan_id,
                http: send_http,
                handler_lock: handle_lock,
            },
        );
        m.content(&format!("Joined {}", connect_to.mention()));
    } else {
        m.content("Error joining the channel");
    }
    m
}

pub struct TrackEndNotifier {
    chan_id: ChannelId,
    http: Arc<Http>,
    handler_lock: Arc<Mutex<Call>>,
}

#[async_trait]
impl VoiceEventHandler for TrackEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            let skipped_meta = match track_list.first() {
                Some((_, handle)) => handle.metadata().clone(),
                None => {
                    error!("couldn't get ended song");
                    return None;
                }
            };
            let handler = self.handler_lock.lock().await;
            if let Some(np) = handler.queue().current() {
                let metadata = np.metadata();
                if let Err(why) = self
                    .chan_id
                    .send_message(&self.http, |m| {
                        _create_skip_embed(m, Some(metadata.clone()), skipped_meta);
                        m
                    })
                    .await
                {
                    error!("Error sending message: {:?}", why);
                }
            } else {
                if let Err(why) = self
                    .chan_id
                    .send_message(&self.http, |m| {
                        _create_skip_embed(m, None, skipped_meta);
                        m
                    })
                    .await
                {
                    error!("Error sending message: {:?}", why);
                }
            }
        }

        None
    }
}

fn _hyperlink_song(data: &Metadata) -> String {
    let mut finished_song = "[".to_string();
    if let Some(title) = &data.title {
        finished_song.push_str(title);
    }

    finished_song.push_str(" - ");

    if let Some(artist) = &data.artist {
        finished_song.push_str(artist);
    }

    finished_song.push_str("](");

    if let Some(link) = &data.source_url {
        finished_song.push_str(link);
    }

    finished_song.push_str(")");

    finished_song
}

fn _create_skip_embed(m: &mut CreateMessage, np: Option<Metadata>, sp: Metadata) {
    m.embed(|e| {
        e.field("Skipped", _hyperlink_song(&sp), false);
        if let Some(meta) = np {
            e.field("Now Playing", _hyperlink_song(&meta), false);
            let duration = match meta.duration {
                Some(duration) => {
                    if duration.as_secs() == 0 {
                        "Live".to_string()
                    } else {
                        format_duration(duration).to_string()
                    }
                }
                None => "Live".to_string(),
            };
            e.field("Duration", duration, true);
            if let Some(t) = meta.thumbnail {
                e.image(t);
            }
        } else {
            if let Some(t) = sp.thumbnail {
                e.image(t);
            }
        }
        e
    });
}

fn _duration_format(duration: Option<Duration>) -> String {
    if let Some(d) = duration {
        if d != Duration::default() {
            return d.as_secs().to_string();
        }
    }
    "Live".to_string()
}
