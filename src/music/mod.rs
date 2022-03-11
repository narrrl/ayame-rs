pub mod join;
pub mod leave;

use std::sync::Arc;

use tracing::error;

use async_trait::async_trait;
use poise::{
    serenity_prelude::{
        ChannelId, Context as SerenityContext, CreateMessage, GuildId, Member, Message, Result,
        UserId,
    },
    Context,
};
use songbird::{Event, EventContext, EventHandler, Songbird, SongbirdKey};

use crate::music::leave::leave;

pub struct MusicContext<'a, U, E>
where
    U: Send + Sync,
    E: Send + Sync,
{
    pub channel_id: ChannelId,
    pub guild_id: GuildId,
    pub author_id: UserId,
    pub ctx: Context<'a, U, E>,
}

impl<'a, U, E> MusicContext<'a, U, E>
where
    U: Send + Sync,
    E: Send + Sync,
{
    pub async fn send<'b, F>(self, f: F) -> Result<Message>
    where
        for<'c> F: FnOnce(&'c mut CreateMessage<'b>) -> &'c mut CreateMessage<'b>,
    {
        self.channel_id
            .send_message(&self.ctx.discord().http, f)
            .await
    }

    pub fn serenity(&'a self) -> &'a SerenityContext {
        self.ctx.discord()
    }
}

struct TimeoutHandler<'a, U, E>
where
    U: Send + Sync,
    E: Send + Sync,
{
    pub ctx: MusicContext<'a, U, E>,
}

#[async_trait]
impl<'a, U, E> EventHandler for TimeoutHandler<'a, U, E>
where
    U: Send + Sync,
    E: Send + Sync,
{
    async fn act(&self, _: &EventContext<'_>) -> Option<Event> {
        let channel = match self
            .ctx
            .serenity()
            .cache
            .guild_channel(self.ctx.channel_id.0)
        {
            Some(channel) => channel,
            None => return None,
        };

        let members = match channel.members(&self.ctx.ctx.discord().cache).await {
            Ok(mems) => mems,
            Err(why) => {
                error!("failed to get members of voice channel: {:?}", why);
                return None;
            }
        };

        if self.is_alone(&members) {
            if let Err(why) = leave(&self.ctx).await {
                error!("leaving voice channel returned error: {:?}", why);
            }
            Some(Event::Cancel)
        } else {
            None
        }
    }
}

impl<'a, U, E> TimeoutHandler<'a, U, E>
where
    U: Send + Sync,
    E: Send + Sync,
{
    fn is_alone(&self, members: &Vec<Member>) -> bool {
        for mem in members.iter() {
            if !mem.user.bot {
                return false;
            }
        }
        true
    }
}

pub async fn get<U, E>(ctx: &Context<'_, U, E>) -> Option<Arc<Songbird>> {
    let data = ctx.discord().data.read().await;
    data.get::<SongbirdKey>().cloned()
}
