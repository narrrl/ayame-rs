use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::framework::standard::CommandResult;

//function that gets called for the ping command
pub async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    msg.reply(&ctx.http, "Pong!").await?;

    Ok(())
}