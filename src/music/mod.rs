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

use crate::{music::leave::leave, Context as PoiseContext, Data, Error};

#[derive(Clone)]
pub struct MusicContext<'a> {
    pub channel_id: ChannelId,
    pub guild_id: GuildId,
    pub author_id: UserId,
    pub ctx: Context<'a, Data, Error>,
}

impl Copy for MusicContext<'_> {}

impl<'a> MusicContext<'a> {
    pub async fn send<'b, F>(&self, f: F) -> Result<Message>
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

struct TimeoutHandler<'a> {
    pub mtx: MusicContext<'a>,
}

#[async_trait]
impl EventHandler for TimeoutHandler<'_> {
    async fn act(&self, _: &EventContext<'_>) -> Option<Event> {
        let channel = match self
            .mtx
            .serenity()
            .cache
            .guild_channel(self.mtx.channel_id.0)
        {
            Some(channel) => channel,
            None => return None,
        };

        let members = match channel.members(&self.mtx.ctx.discord().cache).await {
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

impl TimeoutHandler<'_> {
    fn is_alone(&self, members: &Vec<Member>) -> bool {
        for mem in members.iter() {
            if !mem.user.bot {
                return false;
            }
        }
        true
    }
}

pub async fn get(ctx: &PoiseContext<'_>) -> Option<Arc<Songbird>> {
    let data = ctx.discord().data.read().await;
    data.get::<SongbirdKey>().cloned()
}
