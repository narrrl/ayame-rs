use serenity::{
    async_trait,
    builder::{CreateEmbed, CreateMessage},
    client::{Cache, Context},
    http::Http,
    model::{
        guild::Guild, id::UserId, misc::Mentionable, prelude::ChannelId,
        prelude::GuildId as SerenityGuildId,
    },
    prelude::Mutex,
};
use songbird::{
    driver::Bitrate,
    id::GuildId,
    input::{Metadata, Restartable},
    tracks::{PlayMode, TrackHandle},
    Call, CoreEvent, Event, EventContext, EventHandler as VoiceEventHandler, Songbird, TrackEvent,
};
use std::{ops::Sub, sync::Arc, time::Duration};

use tracing::{error, info};

use crate::model::discord_utils::*;

pub const DEFAULT_BITRATE: i32 = 128_000;

pub async fn stop(ctx: &Context, guild_id: SerenityGuildId) -> CreateEmbed {
    let mut e = default_embed();

    let manager = _get_songbird(ctx).await;

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let _ = queue.stop();

        e.title("Queue cleared.");
    } else {
        set_defaults_for_error(&mut e, "Not in a voice channel to play in");
    }
    e
}

pub async fn skip(ctx: &Context, guild_id: SerenityGuildId) -> CreateEmbed {
    let mut e = default_embed();

    let manager = _get_songbird(ctx).await;

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let _ = queue.skip();
    } else {
        set_defaults_for_error(&mut e, "Not in a voice channel to play in");
    }
    e
}

pub async fn mute(ctx: &Context, guild_id: SerenityGuildId) -> CreateEmbed {
    let mut e = default_embed();

    let manager = _get_songbird(ctx).await;

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            set_defaults_for_error(&mut e, "not in a voice channel");
            return e;
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_mute() {
        if let Err(why) = handler.mute(false).await {
            set_defaults_for_error(&mut e, &format!("Failed: {:?}", why));
            return e;
        }

        e.title("Unmuted");
    } else {
        if let Err(why) = handler.mute(true).await {
            set_defaults_for_error(&mut e, &format!("Failed: {:?}", why));
        } else {
            e.title("Now muted");
        }
    }
    e
}
///
/// joins the the current channel of the message author
///
pub async fn join(
    ctx: &Context,
    guild: Guild,
    author_id: UserId,
    chan_id: ChannelId,
) -> CreateEmbed {
    let mut e = default_embed();
    // get guild id the message was send in
    let guild_id = guild.id;

    // find the voice channel of the author
    // None when author is in no channel
    let channel_id = guild
        .voice_states
        .get(&author_id)
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
    let send_http = ctx.http.clone();
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
        set_defaults_for_error(&mut e, "couldn't join the channel");
    }
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
        Event::Core(CoreEvent::ClientDisconnect),
        LeaveWhenAlone {
            chan_id,
            cache: ctx.cache.clone(),
            http: send_http,
            guild_id: GuildId::from(guild_id),
            manager,
        },
    );
    drop(handle);
    e.description(&format!("Joined {}", connect_to.mention()));
    e
}

pub async fn play_pause(ctx: &Context, guild_id: SerenityGuildId) -> CreateEmbed {
    let mut e = default_embed();
    let manager = _get_songbird(ctx).await;

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let track = match &handler.queue().current() {
            Some(info) => info.clone(),
            None => {
                set_defaults_for_error(&mut e, "nothing is playing");
                return e;
            }
        };

        let is_playing = match track.get_info().await {
            Ok(info) => info.playing == PlayMode::Play,
            Err(_) => false,
        };

        if is_playing {
            if let Err(why) = track.pause() {
                set_defaults_for_error(&mut e, &format!("couldn't pause track {:#?}", why));
            }
        } else {
            if let Err(why) = track.play() {
                set_defaults_for_error(&mut e, &format!("couldn't resume track {:#?}", why));
            }
        }
    } else {
        set_defaults_for_error(&mut e, "Not in a voice channel to play in");
    }
    e
}

pub async fn leave(ctx: &Context, guild_id: SerenityGuildId) -> CreateEmbed {
    let mut e = default_embed();
    let manager = _get_songbird(ctx).await;
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(why) = manager.remove(guild_id).await {
            set_defaults_for_error(&mut e, &format!("Failed: {:?}", why));
        }

        e.title("Left voice channel");
    } else {
        set_defaults_for_error(&mut e, "Not in a voice channel");
    }
    e
}
///
/// deafens bot
///
pub async fn deafen(ctx: &Context, guild_id: SerenityGuildId) -> CreateEmbed {
    // we take our default embed
    let mut e = default_embed();

    // the songbird manager for the current call
    let manager = _get_songbird(ctx).await;
    // get the lock to the call
    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            set_defaults_for_error(&mut e, "not in a voice channel");
            return e;
        }
    };

    // lock the call
    let mut handler = handler_lock.lock().await;

    // check if the bot is already deafened
    if handler.is_deaf() {
        if let Err(why) = handler.deafen(false).await {
            set_defaults_for_error(&mut e, &format!("Failed: {:?}", why));
            return e;
        }

        e.title("Undeafened");
    } else {
        // deafen and let the user know if anything goes horribly wrong
        if let Err(why) = handler.deafen(true).await {
            set_defaults_for_error(&mut e, &format!("failed to deafen {:?}", why));
            return e;
        }

        e.title("Deafened");
    }
    drop(handler);
    e
}

///
/// queues the given link to the song queue of the current call
///
/// also does directly play if its the first song in queue and
/// basically sends the [`now_playing`] command to inform the user
/// that the song started playing
pub async fn play(ctx: &Context, guild_id: SerenityGuildId, url: String) -> CreateEmbed {
    let mut e = default_embed();
    // check if its actually a url
    // TODO: implement yt-search with search terms
    if !url.starts_with("http") {
        set_defaults_for_error(&mut e, "must provide a valid URL");
        return e;
    }

    let manager = _get_songbird(ctx).await;

    // get the current call lock
    if let Some(handler_lock) = manager.get(guild_id) {
        // await the lock
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

        // check if something is playing
        if let Some(_) = handler.queue().current() {
            handler.enqueue_source(source.into());
            e.title(format!(
                "Added song to queue: position {}",
                handler.queue().len()
            ));
        } else {
            // else we queue the song
            handler.enqueue_source(source.into());
            // drop the lock on the call
            drop(handler);
            // to simply invoke the now playing command
            return now_playing(ctx, guild_id).await;
        }
    } else {
        set_defaults_for_error(&mut e, "not in a voice channel to play in");
    }
    return e;
}

///
/// basically sends a nice embed of the current playing song
///
pub async fn now_playing(ctx: &Context, guild_id: SerenityGuildId) -> CreateEmbed {
    let mut e = default_embed();

    let manager = _get_songbird(ctx).await;
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        // get track
        if let Some(track) = handler.queue().current() {
            // field with the name as a hyperlink to the source
            e.field("Now Playing:", _hyperlink_song(track.metadata()), false);
            // field with a nice formatted duration
            let track_time = _duration_format(track.metadata().duration);
            let duration_string = match track.get_info().await {
                Ok(info) => format!("{}/{}", _duration_format(Some(info.position)), track_time),
                Err(_) => track_time,
            };
            e.field("Duration:", duration_string, false);
            // thumbnail url if it exists
            if let Some(image) = &track.metadata().thumbnail {
                e.image(image);
            }
        } else {
            set_defaults_for_error(&mut e, "nothing is playing");
        }
    } else {
        set_defaults_for_error(&mut e, "not in a voice channel to play in");
    }

    // return either error or now playing embed
    return e;
}

pub async fn loop_song(ctx: &Context, guild_id: SerenityGuildId, times: usize) -> CreateEmbed {
    let mut e = default_embed();

    if times > 10 {
        set_defaults_for_error(&mut e, "a song can only be looped 10 times");
        return e;
    }

    let manager = _get_songbird(ctx).await;

    if let Some(handle_lock) = manager.get(guild_id) {
        let handle = handle_lock.lock().await;

        if let Some(track) = handle.queue().current() {
            if let Err(_) = track.loop_for(times) {
                set_defaults_for_error(&mut e, "looping is not supported for this track");
                return e;
            }
            e.field("Now looping", _hyperlink_song(track.metadata()), true);
            e.field("Times", times.to_string(), true);
        } else {
            set_defaults_for_error(&mut e, "nothing is playing");
            return e;
        }
    } else {
        set_defaults_for_error(&mut e, "not in a voice channel to play in");
        return e;
    }
    e
}

pub struct TrackEndNotifier {
    chan_id: ChannelId,
    http: Arc<Http>,
    guild_id: GuildId,
    manager: Arc<Songbird>,
}

// TODO: that definetly needs some structure
// somewhat better, but not the most beautiful code i've ever written
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
                let handler = handler_lock.lock().await;
                let next_playing = handler
                    .queue()
                    .current()
                    .and_then(|track| Some(track.metadata().clone()));
                check_msg(
                    self.chan_id
                        .send_message(&self.http, |m| {
                            _create_next_song_embed(m, next_playing, skipped_meta);
                            m
                        })
                        .await,
                );
            }
        }

        None
    }
}

struct LeaveWhenAlone {
    chan_id: ChannelId,
    cache: Arc<Cache>,
    http: Arc<Http>,
    manager: Arc<Songbird>,
    guild_id: GuildId,
}

#[async_trait]
impl VoiceEventHandler for LeaveWhenAlone {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        // wait some seconds to be sure that the cache is up to date
        std::thread::sleep(Duration::from_secs(3));
        let handle = self
            .manager
            .get(self.guild_id)
            .expect("Couldn't get handle of call");
        // lock handle
        let mut handle = handle.lock().await;
        let channel = self
            .cache
            .guild_channel(handle.current_channel().unwrap().0)
            .await
            .expect("Couldn't get channel");
        let users = channel
            .members(&self.cache)
            .await
            .expect("Couldn't get connected members");
        let mut no_user_connected = true;
        for user in users.iter() {
            if !user.user.bot {
                no_user_connected = false;
                break;
            }
        }
        if no_user_connected {
            handle
                .queue()
                .current()
                .and_then(|track| Some(track.stop()));
            handle.queue().stop();
            handle.stop();
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
                    error!("Failed to remove songbird manager: {:?}", e);
                }
            });
            // create the info message
            let mut embed = default_embed();
            embed.title("Bot disconnected because no one was left in channel");
            // send message
            check_msg(
                self.chan_id
                    .send_message(&self.http, |m| m.set_embed(embed))
                    .await,
            );
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
            return humantime::format_duration(
                // we don't want milliseconds
                d.sub(Duration::from_millis(d.subsec_millis().into())),
            )
            .to_string();
        }
    }
    "Live".to_string()
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
