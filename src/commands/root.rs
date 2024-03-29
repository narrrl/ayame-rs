use crate::{error::Error as AYError, Context, Error};

/// registers or unregisters application commands in this guild or globally
#[poise::command(prefix_command, hide_in_help, owners_only)]
pub async fn register(ctx: Context<'_>, #[flag] global: bool) -> Result<(), Error> {
    poise::builtins::register_application_commands(ctx, global).await?;
    Ok(())
}

/// shutdown the bot
#[poise::command(prefix_command, hide_in_help, owners_only)]
pub async fn shutdown(ctx: Context<'_>) -> Result<(), Error> {
    ctx.framework()
        .shard_manager()
        .lock()
        .await
        .shutdown_all()
        .await;
    Ok(())
}

/// ping the bot for testing
#[poise::command(prefix_command, track_edits, slash_command, hide_in_help)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("pong!").await?;
    Ok(())
}

/// ping but with error for testing
#[poise::command(prefix_command, track_edits, slash_command, hide_in_help)]
pub async fn pingerror(_ctx: Context<'_>) -> Result<(), Error> {
    Err(Box::new(AYError::InvalidInput("test exception")))
}
