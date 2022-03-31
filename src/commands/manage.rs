use crate::error::*;
use crate::utils::guild_only;
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

#[poise::command(
    slash_command,
    category = "server management",
    check = "guild_only",
    required_permissions = "ADMINISTRATOR"
)]
pub(crate) async fn bind(
    ctx: Context<'_>,
    #[description = "the channel that gets bound"] channel: Option<serenity::Channel>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let guild = ctx.guild().ok_or_else(|| Error::Input(NOT_IN_GUILD))?;
    let channel_id = match channel {
        Some(channel) => channel.id(),
        None => ctx.channel_id(),
    };

    let channel = ctx.discord().http.get_channel(channel_id.into()).await?;

    let channel = channel
        .guild()
        .ok_or_else(|| Error::Input(WRONG_CHANNEL_TO_BIND))?;

    match channel.kind {
        serenity::ChannelType::Text => {
            let guild_id = guild.id.0 as i64;
            let bind_id = channel_id.0 as i64;
            sqlx::query!(
                "INSERT OR IGNORE INTO guild_bind (guild_id, bind_id) VALUES (?, ?)",
                guild_id,
                bind_id,
            )
            .execute(&ctx.data().database)
            .await?;
            sqlx::query!(
                "UPDATE guild_bind SET bind_id = ? WHERE guild_id = ?",
                bind_id,
                guild_id
            )
            .execute(&ctx.data().database)
            .await?;
            ctx.say(format!("bound channel {} to bot", channel)).await?;
            Ok(())
        }
        _ => Err(Error::Input(WRONG_CHANNEL_TO_BIND)),
    }
}

#[poise::command(
    slash_command,
    category = "server management",
    check = "guild_only",
    required_permissions = "ADMINISTRATOR"
)]
pub(crate) async fn delete_bind(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let guild = ctx.guild().ok_or_else(|| Error::Input(NOT_IN_GUILD))?;
    let guild_id = guild.id.0 as i64;
    sqlx::query!("DELETE FROM guild_bind WHERE guild_id = ?", guild_id)
        .execute(&ctx.data().database)
        .await?;
    ctx.say("deleted channel bind").await?;
    Ok(())
}

#[poise::command(slash_command, category = "server management", check = "guild_only")]
pub(crate) async fn ping_bind(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let guild = ctx.guild().ok_or_else(|| Error::Input(NOT_IN_GUILD))?;
    let guild_id = guild.id.0 as i64;
    if let Some(bind_id) = get_bound_channel_id(&ctx.data().database, guild_id).await? {
        ctx.say(format!("the channel is <#{}>", bind_id)).await?;
    } else {
        ctx.say("no channel bound").await?;
    }
    Ok(())
}

pub async fn get_bound_channel_id(
    database: &sqlx::SqlitePool,
    guild_id: i64,
) -> Result<Option<u64>, Error> {
    Ok(sqlx::query!(
        "SELECT bind_id FROM guild_bind WHERE guild_id = ?",
        guild_id
    )
    .fetch_optional(database)
    .await?
    .map(|entry| entry.bind_id as u64))
}

pub async fn is_msg_to_keep(
    database: &sqlx::SqlitePool,
    guild_id: i64,
    msg: i64,
) -> Result<bool, Error> {
    Ok(sqlx::query!(
        "SELECT msg_id FROM update_message WHERE guild_id = ?",
        guild_id
    )
    .fetch_optional(database)
    .await?
    .map(|entry| entry.msg_id == msg)
    .unwrap_or(false))
}
