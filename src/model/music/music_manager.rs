use crate::utils::guild_manager;
use serenity::model::channel::Message;
use serenity::prelude::Context;

pub fn test() -> String {
    String::from("Test success!")
}

pub fn play(ctx: &Context, msg: &Message) -> Result<String, String> {


    let Ok(guild) = match guild_manager::get_guild(ctx, msg) {
        Ok(guild) => guild,
        Err(err) => msg.reply(&ctx.http, err)
    };

    Err(String::from("Not implemented"))
}
