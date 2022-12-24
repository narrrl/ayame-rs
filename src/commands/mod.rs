use crate::{error, Context, Error};
use poise::serenity_prelude as serenity;

pub mod admin;
pub mod apex;
pub mod freiburg;
pub mod root;

pub use admin::*;
pub use apex::*;
pub use freiburg::*;
pub use root::*;
pub use uwuifier::*;

/// Show this help menu
#[poise::command(track_edits, slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            show_context_menu_commands: true,
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

/// UwUify text
#[poise::command(slash_command, track_edits)]
pub async fn uwu(
    ctx: Context<'_>,
    #[description = "The text to convert"]
    #[rest]
    text: String,
) -> Result<(), Error> {
    ctx.say(uwuifier::uwuify_str_sse(&text)).await?;
    Ok(())
}

/// UwUify a message
#[poise::command(context_menu_command = "uwuify message")]
pub async fn uwuify(ctx: Context<'_>, msg: serenity::Message) -> Result<(), Error> {
    ctx.say(uwuifier::uwuify_str_sse(&msg.content)).await?;
    Ok(())
}

/// creates an invite to this guild
#[poise::command(slash_command, track_edits, category = "Guild", guild_only, ephemeral)]
pub async fn invite(ctx: Context<'_>) -> Result<(), Error> {
    ctx.guild()
        .ok_or_else(|| error::Error::InvalidInput("not in a guild"))?;
    let inv =
        serenity::Invite::create(&ctx.serenity_context().http, ctx.channel_id(), |f| f).await?;
    ctx.send(|m| m.content(inv.url())).await?;

    Ok(())
}

/// get the avatar of a specific user
#[poise::command(
    slash_command,
    context_menu_command = "get avatar",
    category = "Guild",
    ephemeral,
    guild_only
)]
pub async fn avatar(
    ctx: Context<'_>,
    #[description = "user that you want the avatar from"] user: serenity::User,
) -> Result<(), Error> {
    ctx.say(user.avatar_url().unwrap_or(String::from("No avatar")))
        .await?;
    Ok(())
}
