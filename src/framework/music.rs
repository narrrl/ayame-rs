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
    id::GuildId,
    input::{Metadata, Restartable},
    tracks::TrackHandle,
    Call, Event, EventContext, EventHandler as VoiceEventHandler, Songbird, TrackEvent,
};
use std::{sync::Arc, time::Duration};

use tracing::{error, info};

use crate::model::discord_utils::*;

pub const DEFAULT_BITRATE: i32 = 128_000;

pub async fn join(ctx: &Context, msg: &Message) -> CreateEmbed {
    let mut e = default_embed();
    // get guild the message was send in
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    // find the voice channel of the author
    // None when author is in no channel
    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    // check if author is in any channel
    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            set_defaults_for_error(&mut e, "not in a voice channel");
            return e;
        }
    };

    let manager = _get_songbird(ctx).await;
    match manager.get(guild_id) {
        Some(_) => _switch_channel(&mut e, manager, GuildId::from(guild_id), connect_to, ctx).await,
        None => {
            _new_connection(
                &mut e,
                manager,
                GuildId::from(guild_id),
                connect_to,
                msg.channel_id.clone(),
                ctx,
            )
            .await
        }
    };
    e
}

pub async fn deafen(ctx: &Context, msg: &Message) -> CreateEmbed {
    let mut e = default_embed();
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = _get_songbird(ctx).await;
    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            set_defaults_for_error(&mut e, "not in a voice channel");
            return e;
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_deaf() {
        set_defaults_for_error(&mut e, "already deafened");
    } else {
        if let Err(why) = handler.deafen(true).await {
            set_defaults_for_error(&mut e, &format!("failed to deafen {:?}", why));
            return e;
        }

        e.title("Deafened");
    }

    e
}

pub async fn play(ctx: &Context, msg: &Message, args: &mut Args) -> CreateEmbed {
    let mut e = default_embed();
    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            set_defaults_for_error(&mut e, "must provide a URL to a video or audio");
            return e;
        }
    };

    if !url.starts_with("http") {
        set_defaults_for_error(&mut e, "must provide a valid URL");
        return e;
    }

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = _get_songbird(ctx).await;

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        // Here, we use lazy restartable sources to make sure that we don't pay
        // for decoding, playback on tracks which aren't actually live yet.
        let source = match Restartable::ytdl(url, true).await {
            Ok(source) => source,
            Err(why) => {
                error!("Err starting source: {:?}", why);

                set_defaults_for_error(&mut e, "Error sourcing ffmpeg");
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
        set_defaults_for_error(&mut e, "not in a voice channel to play in");
    }
    return e;
}

pub async fn now_playing(ctx: &Context, msg: &Message) -> CreateEmbed {
    let mut e = default_embed();
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = _get_songbird(ctx).await;
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
            set_defaults_for_error(&mut e, "nothing is playing");
        }
    } else {
        set_defaults_for_error(&mut e, "not in a voice channel to play in");
    }
    return e;
}

pub struct TrackEndNotifier {
    chan_id: ChannelId,
    http: Arc<Http>,
    guild_id: GuildId,
    manager: Arc<Songbird>,
}

// TODO: that definetly needs some structure
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
            if let Some(handler_lock) = self.manager.get(self.guild_id) {
                let mut handler = handler_lock.lock().await;
                if let Some(np) = handler.queue().current() {
                    let metadata = np.metadata();
                    check_msg(
                        self.chan_id
                            .send_message(&self.http, |m| {
                                _create_next_song_embed(m, Some(metadata.clone()), skipped_meta);
                                m
                            })
                            .await,
                    );
                } else {
                    handler.add_global_event(
                        Event::Delayed(Duration::from_secs(300)),
                        AutomaticDisconnect {
                            chan_id: self.chan_id.clone(),
                            http: self.http.clone(),
                            guild_id: self.guild_id.clone(),
                            manager: self.manager.clone(),
                        },
                    );
                    check_msg(
                        self.chan_id
                            .send_message(&self.http, |m| {
                                _create_next_song_embed(m, None, skipped_meta);
                                m
                            })
                            .await,
                    );
                }
            }
        }

        None
    }
}

struct AutomaticDisconnect {
    chan_id: ChannelId,
    http: Arc<Http>,
    manager: Arc<Songbird>,
    guild_id: GuildId,
}

#[async_trait]
impl VoiceEventHandler for AutomaticDisconnect {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        if let Some(handle) = self.manager.get(self.guild_id) {
            // lock handle
            let handle = handle.lock().await;
            // check if bot is idling by locking at the current song
            // if none trigger disconnect
            if let None = handle.queue().current() {
                // get queue to stop it
                let queue = handle.queue();
                let _ = queue.stop();
                // create the info message
                let mut embed = default_embed();
                embed.title("Bot disconnected because of inactivity");
                // send message
                check_msg(
                    self.chan_id
                        .send_message(&self.http, |m| m.set_embed(embed))
                        .await,
                );
                // now we have to destroy the guild handle which is behind
                // a Mutex
                let manager = self.manager.clone();
                let guild_id = self.guild_id.clone();
                // we drop our mutexguard to let the handle free
                drop(handle);
                // now we move the remove to a seperate function to delete
                // it in the near future
                tokio::spawn(async move {
                    if let Err(e) = manager.remove(guild_id).await {
                        error!("Failed: {:?}", e);
                    }
                });
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
    let mut e = default_embed();
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

async fn _new_connection(
    e: &mut CreateEmbed,
    manager: Arc<Songbird>,
    guild_id: GuildId,
    connect_to: ChannelId,
    chan_id: ChannelId,
    ctx: &Context,
) {
    let send_http = ctx.http.clone();
    let handle_lock = _connect(e, &manager, guild_id, connect_to, ctx).await;
    let mut handle = handle_lock.lock().await;
    handle.add_global_event(
        Event::Track(TrackEvent::End),
        TrackEndNotifier {
            chan_id,
            http: send_http.clone(),
            guild_id: GuildId::from(guild_id),
            manager: manager.clone(),
        },
    );

    handle.add_global_event(
        Event::Periodic(Duration::from_secs(900), None),
        AutomaticDisconnect {
            chan_id,
            http: send_http,
            guild_id: GuildId::from(guild_id),
            manager: manager.clone(),
        },
    );
    e.description(&format!("Joined {}", connect_to.mention()));
}

async fn _switch_channel(
    e: &mut CreateEmbed,
    manager: Arc<Songbird>,
    guild_id: GuildId,
    connect_to: ChannelId,
    ctx: &Context,
) {
    _connect(e, &manager, guild_id, connect_to, ctx).await;
    e.description(&format!("Joined {}", connect_to.mention()));
}

async fn _connect(
    e: &mut CreateEmbed,
    manager: &Arc<Songbird>,
    guild_id: GuildId,
    connect_to: ChannelId,
    ctx: &Context,
) -> Arc<Mutex<Call>> {
    let (handle_lock, success) = manager.join(guild_id, connect_to).await;

    if let Ok(_channel) = success {
        let mut handle = handle_lock.lock().await;
        let bitrate = match handle.current_channel() {
            Some(channel) => _get_bitrate_for_channel(channel, &ctx.http).await,
            None => DEFAULT_BITRATE,
        };

        handle.set_bitrate(Bitrate::BitsPerSecond(bitrate.clone()));
        info!("setting bitrate {} for guild {}", bitrate, guild_id);
    } else {
        set_defaults_for_error(e, "couldn't join the channel");
    }

    handle_lock
}

pub async fn _get_songbird(ctx: &Context) -> Arc<Songbird> {
    songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
}

async fn _get_current_song(handle_lock: Arc<Mutex<Call>>) -> Option<TrackHandle> {
    let handle = handle_lock.lock().await;
    handle.queue().current()
}

async fn _get_bitrate_for_channel(channel: songbird::id::ChannelId, http: &Arc<Http>) -> i32 {
    match http.get_channel(channel.0).await {
        Ok(ch) => match ch.guild().expect("Only Guilds are supported").bitrate {
            Some(bitrate) => bitrate as i32,
            // returns default bitrate when it was a textchannel
            None => DEFAULT_BITRATE,
        },
        // what ever
        Err(_) => DEFAULT_BITRATE,
    }
}
