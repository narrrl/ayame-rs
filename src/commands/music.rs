use poise::serenity_prelude::Mentionable;
use songbird::input::Restartable;
use tokio::join;

use crate::{
    music::{self, MusicContext},
    utils::guild_only,
    youtube::{Type, YoutubeSearch},
    Context, Error,
};

use crate::error::*;

#[poise::command(
    prefix_command,
    slash_command,
    category = "Music",
    check = "guild_only",
    global_cooldown = 3
)]
pub(crate) async fn join(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx
        .guild()
        .ok_or_else(|| Error::Input(crate::utils::NOT_IN_GUILD))?;

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

    ctx.send(|m| {
        m.content(format!("Joined {}", channel_id.mention()))
            .ephemeral(true)
    })
    .await?;

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    category = "Music",
    check = "guild_only",
    global_cooldown = 3
)]
pub(crate) async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx
        .guild()
        .ok_or_else(|| Error::Input(crate::utils::NOT_IN_GUILD))?;
    let _ = music::leave::leave(&MusicContext {
        ctx: ctx.discord().clone(),
        guild_id: guild.id,
        channel_id: ctx.channel_id(),
        author_id: ctx.author().id,
    })
    .await?;

    ctx.send(|m| m.content("Left voice").ephemeral(true))
        .await?;

    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    category = "Music",
    check = "guild_only",
    global_cooldown = 3
)]
pub(crate) async fn play(
    ctx: Context<'_>,
    #[description = "The search query or the link"]
    #[rest]
    input: String,
) -> Result<(), Error> {
    let guild = ctx
        .guild()
        .ok_or_else(|| Error::Input(crate::utils::NOT_IN_GUILD))?;
    let channel_id = match guild.voice_states.get(&ctx.author().id) {
        Some(state) => match state.channel_id {
            Some(id) => id,
            None => return Err(Error::Input(NOT_IN_VOICE)),
        },
        None => return Err(Error::Input(NOT_IN_VOICE)),
    };
    let config = ctx.data().config.lock().await;
    let mut req = YoutubeSearch::new(config.youtube_api_key());
    drop(config);
    req.set_amount(1).set_filter(Type::VIDEO);
    let res = req.search(&input).await?;
    let song = res
        .results()
        .first()
        .ok_or_else(|| Error::Input(NO_SEARCH_RESULTS))?;
    let source_future = Restartable::ytdl(song.url(), true);
    let call_future = music::get_call_else_join(&ctx, &guild.id, &channel_id);

    let (source, call) = join!(source_future, call_future);
    let (source, call) = (source?, call?);

    let mut call = call.lock().await;
    call.enqueue_source(source.into());
    drop(call);
    ctx.send(|m| {
        m.embed(|e| {
            e.title(song.title())
                .url(song.url())
                .description("added song")
                .image(song.thumbnail().url())
        })
        .ephemeral(true)
    })
    .await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    category = "Music",
    check = "guild_only",
    global_cooldown = 3
)]
pub(crate) async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx
        .guild()
        .ok_or_else(|| Error::Input(crate::utils::NOT_IN_GUILD))?;

    music::skip::skip(&ctx, &guild.id).await?;

    ctx.send(|m| m.content("Skipped song")).await?;
    Ok(())
}
