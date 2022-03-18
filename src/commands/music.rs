use poise::serenity_prelude::{Mentionable, Message};
use songbird::input::Restartable;
use tokio::join;

use crate::{
    music::{self, hyperlink_song, MusicContext},
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
    global_cooldown = 3,
    ephemeral
)]
pub(crate) async fn join(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
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

    music::join::join(
        &MusicContext {
            songbird: music::get_poise(&ctx).await?,
            guild_id: guild.id,
            channel_id: ctx.channel_id(),
            data: ctx.data().clone(),
            http: ctx.discord().http.clone(),
            cache: ctx.discord().cache.clone(),
        },
        &channel_id,
    )
    .await?;

    ctx.send(|m| m.content(format!("Joined {}", channel_id.mention())))
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
    ctx.defer().await?;
    let guild = ctx
        .guild()
        .ok_or_else(|| Error::Input(crate::utils::NOT_IN_GUILD))?;
    music::leave::leave(&music::get_poise(&ctx).await?, guild.id).await?;

    ctx.send(|m| m.content("Left voice")).await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    category = "Music",
    check = "guild_only",
    global_cooldown = 3,
    ephemeral
)]
pub(crate) async fn play(
    ctx: Context<'_>,
    #[description = "The search query or the link"]
    #[rest]
    input: String,
) -> Result<(), Error> {
    register_and_play(ctx, &input).await
}

#[poise::command(context_menu_command = "Play message content", check = "guild_only")]
pub(crate) async fn play_message_content(
    ctx: Context<'_>,
    #[description = "Message to be played"] message: Message,
) -> Result<(), Error> {
    register_and_play(ctx, &message.content).await
}

async fn register_and_play(ctx: Context<'_>, input: &str) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
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
    let res = req.search(input).await?;
    let song = res
        .results()
        .first()
        .ok_or_else(|| Error::Input(NO_SEARCH_RESULTS))?;
    let source_future = Restartable::ytdl(song.url(), true);
    let call_future = music::get_call_else_join(&ctx, &guild.id, &channel_id);

    let (source, call) = join!(source_future, call_future);
    let (call, source) = (call?, source?);

    let track = music::play::play(&call, source.into()).await?;

    let uuid = track.uuid();

    let user_id = ctx.author().id;

    let mut requests = ctx.data().song_queues.lock().await;
    requests.insert(uuid, user_id);
    drop(requests);

    ctx.say(format!("Added song: {}", hyperlink_song(track.metadata())))
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
    ctx.defer().await?;
    let guild = ctx
        .guild()
        .ok_or_else(|| Error::Input(crate::utils::NOT_IN_GUILD))?;

    music::skip::skip(&ctx, &guild.id).await?;

    ctx.send(|m| m.content("Skipped song")).await?;
    Ok(())
}
