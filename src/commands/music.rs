use std::sync::Arc;

use poise::serenity_prelude::{self as serenity, Mentionable, Message};
use regex::Regex;

use crate::{
    music::{self, embed_song_for_menu, MusicContext},
    utils::{
        cancel, guild_only, next_page, prev_page, select_page, Control, MenuComponent, SelectMenu,
        SelectMenuOptions,
    },
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
pub(crate) async fn search(
    ctx: Context<'_>,
    #[description = "Search term for youtube"]
    #[rest]
    input: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let config = ctx.data().config.lock().await;
    let mut req = YoutubeSearch::new(config.youtube_api_key());
    drop(config);
    req.set_amount(5).set_filter(Type::VIDEO);
    let res = req.search(&input).await?;
    let results = res.results();
    if results.is_empty() {
        return Err(Error::Input(NO_SEARCH_RESULTS));
    }
    let first_row = vec![
        Control::new(
            MenuComponent::button("previous", |b| {
                b.style(serenity::ButtonStyle::Primary).label("previous")
            }),
            Arc::new(|m| Box::pin(prev_page(m))),
        ),
        Control::new(
            MenuComponent::button("next", |b| {
                b.style(serenity::ButtonStyle::Primary).label("next")
            }),
            Arc::new(|m| Box::pin(next_page(m))),
        ),
    ];
    let sec_row = vec![
        Control::new(
            MenuComponent::button("play", |b| {
                b.style(serenity::ButtonStyle::Success).label("play")
            }),
            Arc::new(|m| Box::pin(select_page(m))),
        ),
        Control::new(
            MenuComponent::button("cancel", |b| {
                b.style(serenity::ButtonStyle::Danger).label("cancel")
            }),
            Arc::new(|m| Box::pin(cancel(m))),
        ),
    ];
    let options = SelectMenuOptions::new(0, 120, None, vec![first_row, sec_row], true);
    let pages = results.iter().map(|r| embed_song_for_menu(&r)).collect();
    let menu = SelectMenu::new(&ctx, &pages, options)?;
    let (i, _) = menu.run().await?;
    music::play::register_and_play(
        ctx,
        results
            .get(i)
            .ok_or_else(|| Error::Input(NO_SEARCH_RESULTS))?
            .url(),
    )
    .await
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
    let re = Regex::new(
        r"(http://www\.|https://www\.|http://|https://)?[a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,5}(:[0-9]{1,5})?(/.*)?",
    )?;
    if re.is_match(input.trim()) {
        return music::play::register_and_play(ctx, input).await;
    }
    let config = ctx.data().config.lock().await;
    let mut req = YoutubeSearch::new(config.youtube_api_key());
    drop(config);
    req.set_amount(1).set_filter(Type::VIDEO);
    let res = req.search(&input).await?;
    let song = res
        .results()
        .first()
        .ok_or_else(|| Error::Input(NO_SEARCH_RESULTS))?;
    music::play::register_and_play(ctx, song.url()).await
}

#[poise::command(context_menu_command = "play message", check = "guild_only")]
pub(crate) async fn play_message_content(
    ctx: Context<'_>,
    #[description = "Message to be played"] message: Message,
) -> Result<(), Error> {
    let re = Regex::new(
        r"(http://www\.|https://www\.|http://|https://)?[a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,5}(:[0-9]{1,5})?(/.*)?",
    )?;
    if re.is_match(&message.content.trim()) {
        return music::play::register_and_play(ctx, message.content).await;
    }
    let config = ctx.data().config.lock().await;
    let mut req = YoutubeSearch::new(config.youtube_api_key());
    drop(config);
    req.set_amount(1).set_filter(Type::VIDEO);
    let res = req.search(&message.content).await?;
    let song = res
        .results()
        .first()
        .ok_or_else(|| Error::Input(NO_SEARCH_RESULTS))?;
    music::play::register_and_play(ctx, song.url()).await
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
