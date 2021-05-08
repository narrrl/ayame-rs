use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::model::music::music_manager;

#[command]
async fn test(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(&ctx.http, music_manager::test()).await?;

    Ok(())
}

#[command]
async fn play(ctx: &Context, msg: &Message) -> CommandResult {
    let result = music_manager::play(&ctx, &msg);

    match result {
        Ok(m) => msg.reply(&ctx.http, m).await?,
        Err(e) => msg.reply(&ctx.http, e).await?,
    };

    Ok(())
}
