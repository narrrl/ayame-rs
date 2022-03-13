pub mod join;
pub mod leave;
pub mod now_playing;
pub mod skip;

use std::sync::Arc;

use tracing::error;

use async_trait::async_trait;
use poise::serenity_prelude::{
    self as serenity, ChannelId, Context as SerenityContext, GuildId, Mutex,
};
use songbird::{Call, Event, EventContext, EventHandler, Songbird, SongbirdKey};

use crate::{music::leave::leave, Context as PoiseContext, Error};

use crate::error::*;

#[derive(Clone)]
pub struct MusicContext {
    pub channel_id: serenity::ChannelId,
    pub guild_id: serenity::GuildId,
    pub author_id: serenity::UserId,
    pub ctx: SerenityContext,
}

impl MusicContext {
    #[allow(dead_code)]
    pub async fn send<'b, F>(&self, f: F) -> serenity::Result<serenity::Message>
    where
        for<'c> F:
            FnOnce(&'c mut serenity::CreateMessage<'b>) -> &'c mut serenity::CreateMessage<'b>,
    {
        self.channel_id.send_message(&self.ctx.http, f).await
    }
}

struct TimeoutHandler {
    pub mtx: MusicContext,
}

#[async_trait]
impl EventHandler for TimeoutHandler {
    async fn act(&self, _: &EventContext<'_>) -> Option<Event> {
        let songbird = match get_serenity(&self.mtx.ctx).await {
            Ok(sb) => sb,
            Err(why) => {
                error!("{}", why);
                return Some(Event::Cancel);
            }
        };

        let voice_id = match songbird.get(self.mtx.guild_id.0) {
            Some(call) => {
                let call = call.lock().await;
                match call.current_channel() {
                    Some(ch) => ch,
                    None => return Some(Event::Cancel),
                }
            }
            None => return Some(Event::Cancel),
        };
        let channel = match self.mtx.ctx.cache.guild_channel(voice_id.0) {
            Some(channel) => channel,
            None => return None,
        };

        let members = match channel.members(&self.mtx.ctx.cache).await {
            Ok(mems) => mems,
            Err(why) => {
                error!("failed to get members of voice channel: {:?}", why);
                return None;
            }
        };

        if self.is_alone(&members) {
            if let Err(why) = leave(&self.mtx).await {
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

pub struct TrackNotifier {
    pub mtx: MusicContext,
}

#[async_trait]
impl EventHandler for TrackNotifier {
    async fn act(&self, event: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(_) = event {}
        None
    }
}

#[allow(dead_code)]
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
                    ctx: ctx.discord().clone(),
                    guild_id: *guild_id,
                    channel_id: ctx.channel_id(),
                    author_id: ctx.author().id,
                },
                channel_id,
            )
            .await
        }
    }
}
