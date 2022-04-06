pub mod join;
pub mod leave;
pub mod now_playing;
pub mod play;
pub mod skip;

use std::sync::Arc;
use std::time::Duration;

use songbird::input::Metadata;
use songbird::tracks::{TrackHandle, TrackQueue};

use async_trait::async_trait;
use poise::serenity_prelude::{
    self as serenity, Cache, ChannelId, Context as SerenityContext, CreateEmbed, GuildId, Http,
    Mutex, User,
};
use songbird::{Call, Event, EventContext, EventHandler, Songbird, SongbirdKey};

use crate::commands::manage::get_bound_channel_id;
use crate::utils::{check_result_ayame, Bar};
use crate::youtube::YoutubeResult;
use crate::{Context as PoiseContext, Error};

use crate::{error::*, Data};

#[derive(Clone)]
pub struct MusicContext {
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
        let embed = get_now_playing_embed(&self.mtx.data, &current, queue, &self.mtx.cache).await;
        let msgs = self.mtx.data.playing_messages.lock().await;
        let msg_id = match msgs.get(&self.mtx.guild_id.into()) {
            Some(msg_id) => *msg_id.as_u64(),
            None => return None,
        };
        let channel_id =
            match get_bound_channel_id(&self.mtx.data.database, *self.mtx.guild_id.as_u64() as i64)
                .await
            {
                Ok(Some(ch_id)) => ch_id,
                _ => return None,
            };
        let mut msg = match self.mtx.http.get_message(channel_id, msg_id).await {
            Ok(msg) => msg,
            Err(_) => return None,
        };
        check_result_ayame(self.edit_message(&embed, &mut msg).await);
        None
    }
}

impl NotificationHandler {
    async fn edit_message(
        &self,
        embed: &CreateEmbed,
        message: &mut serenity::Message,
    ) -> Result<(), Error> {
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
}

pub async fn get_now_playing_embed(
    data: &Data,
    current: &Option<TrackHandle>,
    queue: &TrackQueue,
    cache: &Arc<Cache>,
) -> CreateEmbed {
    match current {
        Some(current) => {
            let user_map = data.song_queues.lock().await;
            let user = match user_map.get(&current.uuid()) {
                Some(id) => id.to_user_cached(cache).await,
                None => None,
            };
            embed_upcoming(&queue, embed_song(&current, user).await)
        }
        None => {
            let mut e = CreateEmbed::default();
            e.title("Nothing is playing!");
            e
        }
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
        None => join::join(ctx, guild_id, channel_id).await,
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

pub fn embed_upcoming(queue: &TrackQueue, mut embed: CreateEmbed) -> CreateEmbed {
    let text = queue
        .current_queue()
        .iter()
        .take(6)
        .enumerate()
        .skip(1)
        .filter_map(|(i, s)| match &s.metadata().title {
            Some(title) => Some(format!("{}. {}\n", i, title)),
            None => None,
        })
        .collect::<String>();
    if text.is_empty() {
        return embed;
    }
    embed.field("Upcoming:", format!("```\n{}```", text), false);
    embed
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
                pos_icon: String::from("🔴"),
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
    e.title(song.title())
        .url(song.channel_url())
        .image(song.thumbnail().url());
    e
}
