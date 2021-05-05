use serenity::model::channel::GuildChannel;
use serenity::model::prelude::{ChannelId, Message};
use serenity::prelude::Context;

pub async fn get_guild(ctx: &Context, msg: &Message) -> Result<GuildChannel, String> {
    //get the id of the channel the message was sent in
    let channel_id: ChannelId = msg.channel_id

    //get the channel or an error
    if let Ok(channel) = ctx.http.get_channel(channel_id.0) {
        //get the guild of the channel or an error
        match channel.guild() {
            None => Err(format!("Couldn't get guild for associated channel")),
            Some(guild) => Ok(guild),
        }
    }

    Err(format!("Couldn't find channel"))
}
