use crate::ShardManagerContainer;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::{
    client::bridge::gateway::ShardId,
    framework::standard::{macros::command, CommandResult},
};

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let shard_manager = match data.get::<ShardManagerContainer>() {
        Some(shard_manager) => shard_manager,
        None => {
            msg.reply(ctx, "There was a problem getting the shard manager")
                .await?;

            return Ok(());
        }
    };
    let manager = shard_manager.lock().await;
    let runners = manager.runners.lock().await;

    let runner = match runners.get(&ShardId(ctx.shard_id)) {
        Some(runner) => runner,
        None => {
            msg.reply(ctx, "No shard found").await?;

            return Ok(());
        }
    };

    match runner.latency {
        Some(latency) => {
            msg.reply(&ctx.http, &format!("Pong! {:?}", latency))
                .await?
        }
        None => msg.reply(&ctx.http, "Pong!").await?,
    };
    Ok(())
}
