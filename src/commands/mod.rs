use crate::{error, Context, Error};
use poise::serenity_prelude as serenity;

pub mod admin;

#[poise::command(prefix_command, slash_command, track_edits, category = "General")]
pub(crate) async fn uwu(
    ctx: Context<'_>,
    #[description = "The text to convert"]
    #[rest]
    text: String,
) -> Result<(), Error> {
    ctx.say(uwuifier::uwuify_str_sse(&text)).await?;
    Ok(())
}

#[poise::command(context_menu_command = "uwuify message")]
pub(crate) async fn uwuify(ctx: Context<'_>, msg: serenity::Message) -> Result<(), Error> {
    ctx.say(uwuifier::uwuify_str_sse(&msg.content)).await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    track_edits,
    category = "Guild",
    guild_only,
    ephemeral
)]
pub(crate) async fn invite(ctx: Context<'_>) -> Result<(), Error> {
    ctx.guild()
        .ok_or_else(|| error::Error::InvalidInput("not in a guild"))?;
    let inv = serenity::Invite::create(&ctx.discord().http, ctx.channel_id(), |f| f).await?;
    ctx.send(|m| m.content(inv.url())).await?;

    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    context_menu_command = "get avatar",
    category = "General",
    ephemeral,
    guild_only
)]
pub(crate) async fn avatar(
    ctx: Context<'_>,
    #[description = "user that you want the avatar from"] user: serenity::User,
) -> Result<(), Error> {
    ctx.say(user.avatar_url().unwrap_or(String::from("No avatar")))
        .await?;
    Ok(())
}

/// Registers or unregisters application commands in this guild or globally
#[poise::command(prefix_command, hide_in_help, owners_only)]
pub(crate) async fn register(ctx: Context<'_>, #[flag] global: bool) -> Result<(), Error> {
    poise::builtins::register_application_commands(ctx, global).await?;
    Ok(())
}
