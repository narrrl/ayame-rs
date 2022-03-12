pub mod join;
pub mod leave;

use std::sync::Arc;

use tracing::error;

use async_trait::async_trait;
use poise::serenity_prelude::{self as serenity, Context as SerenityContext};
use songbird::{Event, EventContext, EventHandler, Songbird, SongbirdKey};

use crate::{music::leave::leave, Context as PoiseContext};

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
        let channel = match self.mtx.ctx.cache.guild_channel(self.mtx.channel_id.0) {
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

#[allow(dead_code)]
pub async fn get_poise(ctx: &PoiseContext<'_>) -> Option<Arc<Songbird>> {
    let data = ctx.discord().data.read().await;
    data.get::<SongbirdKey>().cloned()
}

pub async fn get_serenity(ctx: &SerenityContext) -> Option<Arc<Songbird>> {
    let data = ctx.data.read().await;
    data.get::<SongbirdKey>().cloned()
}
