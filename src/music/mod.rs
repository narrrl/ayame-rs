pub mod join;
pub mod leave;
pub mod play;
pub mod skip;

use std::sync::Arc;
use std::time::Duration;

use songbird::input::Metadata;
use songbird::tracks::TrackHandle;
use tracing::error;

use async_trait::async_trait;
use poise::serenity_prelude::{
    self as serenity, Cache, ChannelId, Context as SerenityContext, CreateEmbed, GuildId, Http,
    Message, MessageId, Mutex, User,
};
use songbird::{Call, Event, EventContext, EventHandler, Songbird, SongbirdKey};

use crate::utils::{check_result_ayame, Bar};
use crate::youtube::{hyperlink_result, YoutubeResult};
use crate::{music::leave::leave, Context as PoiseContext, Error};

use crate::{error::*, Data};

#[derive(Clone)]
pub struct MusicContext {
    // the text channel
    pub channel_id: serenity::ChannelId,
    // the guild in which the bot plays music
    pub guild_id: serenity::GuildId,
    // cache for faster operations
    pub cache: Arc<Cache>,
    // http for sending messages and stuff
    pub http: Arc<Http>,
    // data because, idk maybe in the future
    pub data: Data,
    // songbird for call/tracks
    pub songbird: Arc<Songbird>,
}

struct NotificationHandler {
    pub mtx: MusicContext,
}

impl NotificationHandler {
    async fn send_new_message(
        &self,
        embed: &CreateEmbed,
        old_msg: Option<MessageId>,
    ) -> Result<Message, Error> {
        if let Some(old_msg) = old_msg {
            self.delete_old(old_msg).await;
        }
        Ok(self
            .mtx
            .channel_id
            .send_message(&self.mtx.http, |m| {
                m.embed(|e| {
                    e.clone_from(embed);
                    e
                })
            })
            .await?)
    }

    async fn edit_message(
        &self,
        embed: &CreateEmbed,
        message: &mut Message,
        old_msg: Option<MessageId>,
    ) -> Result<(), Error> {
        if let Some(old_msg) = old_msg {
            self.delete_old(old_msg).await;
        }
        message
            .edit(&self.mtx.http, |m| {
                m.embed(|e| {
                    e.clone_from(embed);
                    e
                })
            })
            .await?;
        Ok(())
    }

    async fn delete_old(&self, old_msg: MessageId) {
        let _ = self
            .mtx
            .http
            .delete_message(self.mtx.channel_id.0, old_msg.0)
            .await;
    }

    async fn is_newest<'a>(&'a self, msg_id: &MessageId) -> bool {
        match self.mtx.http.get_messages(self.mtx.channel_id.0, "").await {
            Ok(msgs) => msgs
                .first()
                .and_then(|m| Some(&m.id == msg_id))
                .unwrap_or(false),
            Err(_) => false,
        }
    }
}

#[async_trait]
impl EventHandler for NotificationHandler {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let songbird = &self.mtx.songbird;

        let call = match songbird.get(self.mtx.guild_id) {
            Some(call) => call,
            None => return Some(Event::Cancel),
        };

        let call = call.lock().await;

        let queue = call.queue();
        let current = queue.current();
        drop(call);

        let mut song_status = self.mtx.data.song_status.lock().await;
        let embed = match current {
            Some(current) => {
                let user_map = self.mtx.data.song_queues.lock().await;
                song_status.insert(self.mtx.guild_id, false);
                let user = match user_map.get(&current.uuid()) {
                    Some(id) => id.to_user_cached(&self.mtx.cache).await,
                    None => None,
                };
                drop(user_map);

                embed_song(&current, user).await
            }
            None => {
                if *song_status.entry(self.mtx.guild_id).or_insert(true) {
                    return None;
                }
                song_status.insert(self.mtx.guild_id, true);
                let mut e = CreateEmbed::default();
                e.title("Nothing is playing!");
                e
            }
        };
        drop(song_status);

        let mut messages_map = self.mtx.data.song_messages.lock().await;
        match messages_map.get_mut(&self.mtx.guild_id) {
            // check if we already have a message
            Some(id) => {
                // get the message from the id
                let mut message = match self.mtx.cache.message(self.mtx.channel_id.0, id.0) {
                    Some(msg) => msg,
                    None => match self.mtx.http.get_message(self.mtx.channel_id.0, id.0).await {
                        Ok(msg) => msg,
                        Err(_) => {
                            messages_map.remove(&self.mtx.guild_id);
                            drop(messages_map);
                            return None;
                        }
                    },
                };
                // either delete message and send new
                // or update old message
                let message = if !self.is_newest(&message.id).await {
                    self.send_new_message(&embed, Some(message.id)).await
                } else {
                    check_result_ayame(self.edit_message(&embed, &mut message, None).await);
                    drop(messages_map);
                    return None;
                };
                if let Ok(msg) = message {
                    messages_map.insert(self.mtx.guild_id, msg.id);
                }
            }
            // else create a message
            None => {
                let message = match self.send_new_message(&embed, None).await {
                    Ok(msg) => msg,
                    Err(_) => {
                        drop(messages_map);
                        return None;
                    }
                };
                messages_map.insert(self.mtx.guild_id, message.id);
            }
        };
        drop(messages_map);
        None
    }
}

struct TimeoutHandler {
    pub mtx: MusicContext,
}

#[async_trait]
impl EventHandler for TimeoutHandler {
    async fn act(&self, _: &EventContext<'_>) -> Option<Event> {
        let voice_id = match self.mtx.songbird.get(self.mtx.guild_id.0) {
            Some(call) => {
                let call = call.lock().await;
                match call.current_channel() {
                    Some(ch) => ch,
                    None => return Some(Event::Cancel),
                }
            }
            None => return Some(Event::Cancel),
        };
        let channel = match self.mtx.cache.guild_channel(voice_id.0) {
            Some(channel) => channel,
            None => return None,
        };

        let members = match channel.members(&self.mtx.cache).await {
            Ok(mems) => mems,
            Err(why) => {
                error!("failed to get members of voice channel: {:?}", why);
                return None;
            }
        };

        if self.is_alone(&members) {
            if let Err(why) = leave(&self.mtx.songbird, self.mtx.guild_id).await {
                error!("leaving voice channel returned error: {:?}", why);
            }
            Some(Event::Cancel)
        } else {
            None
        }
    }
}

impl TimeoutHandler {
    fn is_alone(&self, members: &Vec<serenity::Member>) -> bool {
        for mem in members.iter() {
            if !mem.user.bot {
                return false;
            }
        }
        true
    }
}

pub async fn get_poise(ctx: &PoiseContext<'_>) -> Result<Arc<Songbird>, Error> {
    get_serenity(ctx.discord()).await
}

pub async fn get_serenity(ctx: &SerenityContext) -> Result<Arc<Songbird>, Error> {
    let data = ctx.data.read().await;
    match data.get::<SongbirdKey>().cloned() {
        Some(songbird) => Ok(songbird),
        None => Err(Error::Failure(FAILD_TO_GET_SONGBIRD)),
    }
}

pub async fn get_call_else_join(
    ctx: &PoiseContext<'_>,
    guild_id: &GuildId,
    channel_id: &ChannelId,
) -> Result<Arc<Mutex<Call>>, Error> {
    let songbird = get_poise(&ctx).await?;

    match songbird.get(guild_id.0) {
        Some(call) => Ok(call),
        None => {
            join::join(
                &MusicContext {
                    songbird: get_poise(&ctx).await?,
                    guild_id: *guild_id,
                    channel_id: ctx.channel_id(),
                    data: ctx.data().clone(),
                    http: ctx.discord().http.clone(),
                    cache: ctx.discord().cache.clone(),
                },
                channel_id,
            )
            .await
        }
    }
}

pub fn hyperlink_song(data: &Metadata) -> String {
    let mut finished_song = "[".to_string();
    if let Some(title) = &data.title {
        finished_song.push_str(title);
    }

    finished_song.push_str("](");

    if let Some(link) = &data.source_url {
        finished_song.push_str(link);
    }

    finished_song.push_str(")");

    finished_song
}

pub fn duration_format(duration: Option<Duration>) -> String {
    if let Some(d) = duration {
        if d != Duration::default() {
            return humantime::format_duration(
                // we don't want milliseconds
                d - Duration::from_millis(d.subsec_millis().into()),
            )
            .to_string();
        }
    }
    "unknown".to_string()
}

pub async fn embed_song(track: &TrackHandle, user: Option<User>) -> CreateEmbed {
    let mut e = CreateEmbed::default();
    let duration = track.metadata().duration;

    let current_duration = match track.get_info().await {
        Ok(info) => Some(info.play_time),
        Err(_) => None,
    };

    let duration_str = format!(
        "{}/{}",
        duration_format(current_duration),
        duration_format(duration)
    );

    let bar = match (current_duration, duration) {
        (Some(cd), Some(d)) => {
            let mut bar = Bar {
                pos_icon: String::from("<a:gandalf_pls:899788418358345809>"),
                line_icon: String::from("="),
                ..Default::default()
            };
            bar.set_len(30);
            bar.set(cd.as_secs() as f64 / d.as_secs() as f64);
            bar.to_string()
        }
        _ => "".to_string(),
    };

    e.field("Now playing:", hyperlink_song(track.metadata()), false)
        .field("Duration:", format!("{}\n{}", duration_str, bar), false);
    if let Some(thumbnail) = &track.metadata().thumbnail {
        e.image(thumbnail);
    }
    if let Some(user) = user {
        e.author(|a| {
            if let Some(avatar) = user.avatar_url() {
                a.icon_url(avatar);
            }
            a.name(user.name)
        });
    }
    e
}

pub fn embed_song_for_menu(song: &YoutubeResult) -> CreateEmbed {
    let mut e = CreateEmbed::default();
    e.author(|a| a.name(song.channel_name()).url(song.channel_url()))
        .image(song.thumbnail().url())
        .description(hyperlink_result(&song));
    e
}
