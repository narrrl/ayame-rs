use poise::serenity_prelude::Mentionable;

use crate::{
    music::{self, MusicContext},
    utils::guild_only,
    Context, Error,
};

pub const NOT_IN_VOICE: &'static str = "not in a voice channel";

#[poise::command(
    prefix_command,
    slash_command,
    category = "Music",
    check = "guild_only"
)]
pub(crate) async fn join(ctx: Context<'_>) -> Result<(), Error> {
    // safe because `guild_only` already tests for guild
    let guild = ctx.guild().unwrap();

    let channel_id = match guild.voice_states.get(&ctx.author().id) {
        Some(state) => match state.channel_id {
            Some(id) => id,
            None => return Err(Error::Input(NOT_IN_VOICE)),
        },
        None => return Err(Error::Input(NOT_IN_VOICE)),
    };

    let _ = music::join::join(
        &MusicContext {
            ctx: ctx.discord().clone(),
            guild_id: guild.id,
            channel_id: ctx.channel_id(),
            author_id: ctx.author().id,
        },
        &channel_id,
    )
    .await?;

    ctx.say(format!("Joined {}", channel_id.mention())).await?;

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    category = "Music",
    check = "guild_only"
)]
pub(crate) async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    // safe because `guild_only` already tests for guild
    let guild = ctx.guild().unwrap();
    let _ = music::leave::leave(&MusicContext {
        ctx: ctx.discord().clone(),
        guild_id: guild.id,
        channel_id: ctx.channel_id(),
        author_id: ctx.author().id,
    })
    .await?;

    ctx.say("Left voice").await?;

    Ok(())
}
