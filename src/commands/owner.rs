use crate::ShardManagerContainer;
use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use tracing::info;

#[command("shutdown")]
#[aliases("shutd", "sh")]
#[description("Shutdown bot")]
#[owners_only]
#[num_args(0)]
async fn shutdown(ctx: &Context, msg: &Message) -> CommandResult {
    info!("Recieved shutdown event");
    let data = ctx.data.read().await;

    if let Some(manager) = data.get::<ShardManagerContainer>() {
        msg.reply(&ctx.http, "Shutting down...").await?;
        manager.lock().await.shutdown_all().await;
    } else {
        msg.reply(
            &ctx.http,
            "Couldn't get the shard manager -> Shutdown canceled",
        )
        .await?;
    }
    Ok(())
}
