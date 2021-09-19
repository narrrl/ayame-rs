use serenity::{
    async_trait,
    builder::{CreateEmbed, CreateMessage},
    client::Context,
    framework::standard::Args,
    http::Http,
    model::{channel::Message, misc::Mentionable, prelude::ChannelId},
    prelude::Mutex,
};
use songbird::{
    driver::Bitrate,
    input::{Metadata, Restartable},
    Call, Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent,
};
use std::{sync::Arc, time::Duration};

use tracing::{error, info};

use crate::model::discord_utils;

pub const DEFAULT_BITRATE: i32 = 128_000;

pub async fn join(ctx: &Context, msg: &Message) -> CreateEmbed {
    let mut e = discord_utils::default_embed();
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            discord_utils::set_defaults_for_error(&mut e, "not in a voice channel");
            return e;
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
        e.description(&format!("Joined {}", connect_to.mention()));
    } else {
        discord_utils::set_defaults_for_error(&mut e, "couldn't joining the channel");
    }
    e
}

pub async fn deafen(ctx: &Context, msg: &Message) -> CreateEmbed {
    let mut e = discord_utils::default_embed();
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            discord_utils::set_defaults_for_error(&mut e, "not in a voice channel");
            return e;
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_deaf() {
        discord_utils::set_defaults_for_error(&mut e, "already deafened");
    } else {
        if let Err(why) = handler.deafen(true).await {
            discord_utils::set_defaults_for_error(&mut e, &format!("failed to deafen {:?}", why));
            return e;
        }

        e.title("Deafened");
    }

    e
}

pub async fn play(ctx: &Context, msg: &Message, args: &mut Args) -> CreateEmbed {
    let mut e = discord_utils::default_embed();
    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            discord_utils::set_defaults_for_error(&mut e, "must provide a URL to a video or audio");
            return e;
        }
    };

    if !url.starts_with("http") {
        discord_utils::set_defaults_for_error(&mut e, "must provide a valid URL");
        return e;
    }

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        // Here, we use lazy restartable sources to make sure that we don't pay
        // for decoding, playback on tracks which aren't actually live yet.
        let source = match Restartable::ytdl(url, true).await {
            Ok(source) => source,
            Err(why) => {
                error!("Err starting source: {:?}", why);

                discord_utils::set_defaults_for_error(&mut e, "Error sourcing ffmpeg");
                return e;
            }
        };

        if let Some(_) = handler.queue().current() {
            handler.enqueue_source(source.into());
            e.title(format!(
                "Added song to queue: position {}",
                handler.queue().len()
            ));
        } else {
            handler.enqueue_source(source.into());
            drop(handler);
            return now_playing(ctx, msg).await;
        }
    } else {
        discord_utils::set_defaults_for_error(&mut e, "not in a voice channel to play in");
    }
    return e;
}

pub async fn now_playing(ctx: &Context, msg: &Message) -> CreateEmbed {
    let mut e = discord_utils::default_embed();
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        if let Some(track) = handler.queue().current() {
            e.field("Now Playing:", _hyperlink_song(track.metadata()), false);
            e.field(
                "Duration:",
                _duration_format(track.metadata().duration),
                false,
            );
            if let Some(image) = &track.metadata().thumbnail {
                e.image(image);
            }
        } else {
            discord_utils::set_defaults_for_error(&mut e, "nothing is playing");
        }
    } else {
        discord_utils::set_defaults_for_error(&mut e, "not in a voice channel to play in");
    }
    return e;
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
                        _create_next_song_embed(m, Some(metadata.clone()), skipped_meta);
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
                        _create_next_song_embed(m, None, skipped_meta);
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

fn _create_next_song_embed(m: &mut CreateMessage, np: Option<Metadata>, sp: Metadata) {
    let mut e = discord_utils::default_embed();
    e.field("Finished:", _hyperlink_song(&sp), false);
    e.field("Duration:", _duration_format(sp.duration), false);
    if let Some(meta) = np {
        e.field("Now Playing:", _hyperlink_song(&meta), false);
        e.field("Duration", _duration_format(meta.duration), false);
        if let Some(t) = meta.thumbnail {
            e.image(t);
        }
    } else {
        if let Some(t) = sp.thumbnail {
            e.image(t);
        }
    }
    m.set_embed(e);
}

fn _duration_format(duration: Option<Duration>) -> String {
    if let Some(d) = duration {
        if d != Duration::default() {
            return humantime::format_duration(d).to_string();
        }
    }
    "Live".to_string()
}
