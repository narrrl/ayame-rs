use crate::{Context, Error};

#[poise::command(prefix_command, owners_only, hide_in_help)]
pub(crate) async fn shutdown(ctx: Context<'_>) -> Result<(), Error> {
    ctx.framework()
        .shard_manager()
        .lock()
        .await
        .shutdown_all()
        .await;
    Ok(())
}

/// Register slash commands in this guild or globally
/// Run with no arguments to register in guild, run with argument "global" to register globally.
#[poise::command(prefix_command, hide_in_help)]
pub(crate) async fn register(ctx: Context<'_>, #[flag] global: bool) -> Result<(), Error> {
    poise::builtins::register_application_commands(ctx, global).await?;
    Ok(())
}

#[poise::command(prefix_command, hide_in_help, owners_only)]
pub(crate) async fn unregister(ctx: Context<'_>) -> Result<(), Error> {
    let commands = match ctx.guild() {
        Some(guild) => {
            ctx.say("Deleting guild application commands").await?;
            guild.get_application_commands(&ctx.discord().http).await?
        }
        None => ctx.discord().http.get_global_application_commands().await?,
    };
    ctx.say(format!("Deleting {} commands...", commands.len()))
        .await?;
    for cmd in commands {
        if let Some(guild) = ctx.guild() {
            ctx.discord()
                .http
                .delete_guild_application_command(guild.id.into(), cmd.id.into())
                .await?;
        } else {
            ctx.discord()
                .http
                .delete_global_application_command(cmd.id.into())
                .await?;
        }
    }
    ctx.say("Done!").await?;
    Ok(())
}
