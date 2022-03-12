use poise::serenity_prelude::Mentionable;

use crate::{
    music::{self, MusicContext},
    Context, Error,
};

#[poise::command(prefix_command, slash_command, category = "Music")]
pub(crate) async fn join(ctx: Context<'_>) -> Result<(), Error> {
    let guild = match ctx.guild() {
        Some(guild) => guild,
        None => {
            ctx.say("only in guilds").await?;
            return Ok(());
        }
    };

    let channel_id = match guild.voice_states.get(&ctx.author().id) {
        Some(state) => match state.channel_id {
            Some(id) => id,
            None => {
                ctx.say("Not in a voice channel").await?;
                return Ok(());
            }
        },
        None => {
            ctx.say("Not in a voice channel").await?;
            return Ok(());
        }
    };

    let _ = music::join::join(
        &ctx,
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

#[poise::command(prefix_command, slash_command, category = "Music")]
pub(crate) async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    let guild = match ctx.guild() {
        Some(guild) => guild,
        None => {
            ctx.say("only in guilds").await?;
            return Ok(());
        }
    };

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
