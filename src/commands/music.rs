use std::sync::Arc;

use poise::serenity_prelude::{self as serenity, ButtonStyle, CreateButton, Mentionable, Message};
use regex::Regex;

use crate::{
    menu::Menu,
    menu::{Control, Cursor, MenuComponent},
    music::{self, embed_song_for_menu, MusicContext},
    utils::{bind_command, guild_only},
    youtube::{Type, YoutubeResult, YoutubeSearch},
    Context, Error,
};

use crate::error::*;

#[poise::command(
    prefix_command,
    slash_command,
    category = "Music",
    check = "bind_command",
    global_cooldown = 3
)]
pub(crate) async fn join(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let guild = ctx.guild().ok_or_else(|| Error::Input(NOT_IN_GUILD))?;

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
    check = "bind_command",
    global_cooldown = 3
)]
pub(crate) async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let guild = ctx.guild().ok_or_else(|| Error::Input(NOT_IN_GUILD))?;
    music::leave::leave(&music::get_poise(&ctx).await?, guild.id).await?;

    ctx.send(|m| m.content("Left voice")).await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    category = "Music",
    check = "bind_command",
    global_cooldown = 3
)]
pub(crate) async fn search(
    ctx: Context<'_>,
    #[description = "Search term for youtube"]
    #[rest]
    input: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let config = &ctx.data().config;
    let color = config.color()?;
    let mut req = YoutubeSearch::new(config.youtube_api_key());
    req.set_amount(5).set_filter(Type::VIDEO);
    let res = req.search(&input).await?;
    let results = res.results();

    let song = results
        .first()
        .ok_or_else(|| Error::Input(NO_SEARCH_RESULTS))?;

    let mut menu = Menu::new(&ctx, Cursor::from(results), |options| {
        options
            .add_row(|row| {
                row.add_button(Control::new(
                    MenuComponent::button("prev", |b: &mut CreateButton| {
                        b.style(ButtonStyle::Primary).label("prev")
                    }),
                    Arc::new(|m, mci| Box::pin(prev(m, mci))),
                ))
                .add_button(Control::new(
                    MenuComponent::button("next", |b: &mut CreateButton| {
                        b.style(ButtonStyle::Primary).label("next")
                    }),
                    Arc::new(|m, mci| Box::pin(next(m, mci))),
                ))
                .add_button(Control::new(
                    MenuComponent::button("play", |b: &mut CreateButton| {
                        b.style(ButtonStyle::Success).label("play")
                    }),
                    Arc::new(|m, mci| Box::pin(select(m, mci))),
                ))
                .add_button(Control::new(
                    MenuComponent::button("cancel", |b: &mut CreateButton| {
                        b.style(ButtonStyle::Danger).label("cancel")
                    }),
                    Arc::new(|m, mci| Box::pin(cancel(m, mci))),
                ))
            })
            .set_post_hook(Arc::new(|m| Box::pin(post_hook(m))))
    });

    menu.run(|mes| {
        mes.embed(|e| {
            e.clone_from(&embed_song_for_menu(song));
            e.color(color);
            e
        })
    })
    .await?;

    let song = menu
        .data
        .current()
        .ok_or_else(|| Error::Input(NO_SEARCH_RESULTS))?;
    music::play::register_and_play(ctx, song.url()).await?;
    Ok(())
}

async fn post_hook(m: &mut Menu<'_, Cursor<'_, YoutubeResult>>) -> Result<(), Error> {
    if let Some(msg_id) = m.msg_id {
        let msg = m
            .ctx
            .discord()
            .http
            .get_message(m.ctx.channel_id().into(), msg_id.into())
            .await?;
        msg.delete(&m.ctx.discord().http).await?;
    }
    Ok(())
}

async fn cancel(
    m: &mut Menu<'_, Cursor<'_, YoutubeResult>>,
    mci: &Arc<serenity::MessageComponentInteraction>,
) -> Result<(), Error> {
    let _ = mci.defer(&m.ctx.discord().http).await;
    Err(Error::Input(EVENT_CANCELED))
}
async fn select(
    m: &mut Menu<'_, Cursor<'_, YoutubeResult>>,
    _mci: &Arc<serenity::MessageComponentInteraction>,
) -> Result<(), Error> {
    m.stop();
    Ok(())
}

async fn next(
    m: &mut Menu<'_, Cursor<'_, YoutubeResult>>,
    mci: &Arc<serenity::MessageComponentInteraction>,
) -> Result<(), Error> {
    let song = m
        .data
        .next()
        .ok_or_else(|| Error::Failure(NO_SEARCH_RESULTS))?;
    let color = m.ctx.data().config.color()?;

    m.update_response(
        |m| {
            m.set_embed({
                let mut e = embed_song_for_menu(song);
                e.color(color);
                e
            })
        },
        mci,
    )
    .await
}
async fn prev(
    m: &mut Menu<'_, Cursor<'_, YoutubeResult>>,
    mci: &Arc<serenity::MessageComponentInteraction>,
) -> Result<(), Error> {
    let song = m
        .data
        .prev()
        .ok_or_else(|| Error::Failure(NO_SEARCH_RESULTS))?;
    let color = m.ctx.data().config.color()?;

    m.update_response(
        |m| {
            m.set_embed({
                let mut e = embed_song_for_menu(song);
                e.color(color);
                e
            })
        },
        mci,
    )
    .await
}

#[poise::command(
    prefix_command,
    slash_command,
    category = "Music",
    check = "bind_command",
    global_cooldown = 3
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
        let track = music::play::register_and_play(ctx, input).await?;
        ctx.say(format!(
            "Added song: {}",
            crate::music::hyperlink_song(track.metadata())
        ))
        .await?;
        return Ok(());
    }
    let config = &ctx.data().config;
    let mut req = YoutubeSearch::new(config.youtube_api_key());
    req.set_amount(1).set_filter(Type::VIDEO);
    let res = req.search(&input).await?;
    let song = res
        .results()
        .first()
        .ok_or_else(|| Error::Input(NO_SEARCH_RESULTS))?;
    let track = music::play::register_and_play(ctx, song.url()).await?;
    ctx.say(format!(
        "Added song: {}",
        crate::music::hyperlink_song(track.metadata())
    ))
    .await?;
    Ok(())
}

#[poise::command(context_menu_command = "play message", check = "guild_only", ephemeral)]
pub(crate) async fn play_message_content(
    ctx: Context<'_>,
    #[description = "Message to be played"] message: Message,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let re = Regex::new(
        r"(http://www\.|https://www\.|http://|https://)?[a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,5}(:[0-9]{1,5})?(/.*)?",
    )?;
    if re.is_match(&message.content.trim()) {
        let track = music::play::register_and_play(ctx, message.content).await?;
        ctx.say(format!(
            "Added song: {}",
            crate::music::hyperlink_song(track.metadata())
        ))
        .await?;
        return Ok(());
    }
    let config = &ctx.data().config;
    let mut req = YoutubeSearch::new(config.youtube_api_key());
    req.set_amount(1).set_filter(Type::VIDEO);
    let res = req.search(&message.content).await?;
    let song = res
        .results()
        .first()
        .ok_or_else(|| Error::Input(NO_SEARCH_RESULTS))?;
    let track = music::play::register_and_play(ctx, song.url()).await?;
    ctx.say(format!(
        "Added song: {}",
        crate::music::hyperlink_song(track.metadata())
    ))
    .await?;
    Ok(())
}

#[poise::command(
    prefix_command,
    slash_command,
    category = "Music",
    check = "bind_command",
    global_cooldown = 3
)]
pub(crate) async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;
    let guild = ctx.guild().ok_or_else(|| Error::Input(NOT_IN_GUILD))?;

    music::skip::skip(&ctx, &guild.id).await?;

    ctx.send(|m| m.content("Skipped song")).await?;
    Ok(())
}
