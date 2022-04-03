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
    ctx.defer().await?;
    let guild = ctx.guild().ok_or_else(|| Error::Input(NOT_IN_GUILD))?;
    let channel_id = match channel {
        Some(channel) => channel.id(),
        None => ctx.channel_id(),
    };

    let channel = ctx.discord().http.get_channel(channel_id.into()).await?;

    let channel = channel
        .guild()
        .ok_or_else(|| Error::Input(WRONG_CHANNEL_TO_BIND))?;
    let guild_id = guild.id.0 as i64;
    let bind_id = channel_id.0 as i64;
    let old_channel = get_bound_channel_id(&ctx.data().database, guild_id).await?;

    match channel.kind {
        serenity::ChannelType::Text => {
            if Some(bind_id) == old_channel.map(|n| n as i64) {
                return Err(Error::Input(CHANNEL_ALREADY_BOUND));
            }
            let msg = channel_id
                .send_message(&ctx.discord().http, |m| m.content("placeholder (muss grad bisl was umschreiben, erstmal keine status message)"))
                .await?;
            let msg_id = msg.id.0 as i64;
            register_msg(&ctx.data().database, guild_id, msg_id).await?;
            bind_channel(&ctx.data().database, guild_id, bind_id).await?;
            // TODO: fix the unregistering and unbinding
            // TODO: remember what wasn't working again
            if let Some(msg) = get_status_msg(&ctx.data().database, guild_id).await? {
                unregister_msg(&ctx.data().database, guild_id, msg as i64).await?;
            }
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
    unbind_channel(&ctx.data().database, guild_id).await?;
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

pub async fn should_be_deleted(
    database: &sqlx::SqlitePool,
    guild_id: i64,
    msg: i64,
) -> Result<bool, Error> {
    Ok(sqlx::query!(
        "SELECT msg_id FROM update_message WHERE guild_id = ?",
        guild_id
    )
    .fetch_all(database)
    .await?
    .iter()
    .any(|entry| entry.msg_id == msg))
}

pub async fn unregister_msg(
    database: &sqlx::SqlitePool,
    guild_id: i64,
    msg_id: i64,
) -> Result<(), Error> {
    sqlx::query!(
        "DELETE FROM update_message WHERE msg_id = ? AND guild_id = ?",
        msg_id,
        guild_id
    )
    .execute(database)
    .await?;
    Ok(())
}

pub async fn register_msg(
    database: &sqlx::SqlitePool,
    guild_id: i64,
    msg_id: i64,
) -> Result<(), Error> {
    sqlx::query!(
        "INSERT INTO update_message (guild_id, msg_id) VALUES (?, ?)",
        guild_id,
        msg_id,
    )
    .execute(database)
    .await?;
    Ok(())
}

pub async fn bind_channel(
    database: &sqlx::SqlitePool,
    guild_id: i64,
    bind_id: i64,
) -> Result<(), Error> {
    sqlx::query!(
        "INSERT OR IGNORE INTO guild_bind (guild_id, bind_id) VALUES (?, ?)",
        guild_id,
        bind_id,
    )
    .execute(database)
    .await?;
    sqlx::query!(
        "UPDATE guild_bind SET bind_id = ? WHERE guild_id = ?",
        bind_id,
        guild_id
    )
    .execute(database)
    .await?;
    Ok(())
}
pub async fn unbind_channel(database: &sqlx::SqlitePool, guild_id: i64) -> Result<(), Error> {
    sqlx::query!("DELETE FROM guild_bind WHERE guild_id = ?", guild_id)
        .execute(database)
        .await?;
    Ok(())
}

pub async fn get_status_msg(
    database: &sqlx::SqlitePool,
    guild_id: i64,
) -> Result<Option<u64>, Error> {
    Ok(sqlx::query!(
        "SELECT msg_id FROM update_message WHERE guild_id = ?",
        guild_id
    )
    .fetch_optional(database)
    .await?
    .map(|entry| entry.msg_id as u64))
}
