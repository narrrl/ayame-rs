use std::sync::Arc;

use serenity::{
    builder::CreateEmbed,
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    http::Http,
    model::{channel::Message, id::ChannelId},
};

use crate::framework;
use crate::model::discord_utils::*;

#[command]
#[only_in(guilds)]
#[description("Deafens the bot")]
#[num_args(0)]
async fn deafen(ctx: &Context, msg: &Message) -> CommandResult {
    _send_response(
        &msg.channel_id,
        &ctx.http,
        framework::music::deafen(ctx, msg).await,
    )
    .await
}

#[command]
#[aliases("j")]
#[only_in(guilds)]
#[description("Makes the bot join your channel")]
#[num_args(0)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let author_id = msg.author.id;
    let chan_id = msg.channel_id;
    _send_response(
        &msg.channel_id,
        &ctx.http,
        framework::music::join(&ctx, guild, author_id, chan_id).await,
    )
    .await
}

#[command]
#[aliases("np")]
#[only_in(guilds)]
#[description("Shows the currently playing song")]
#[num_args(0)]
async fn now_playing(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;
    _send_response(
        &msg.channel_id,
        &ctx.http,
        framework::music::now_playing(ctx, guild_id).await,
    )
    .await
}

#[command("play_pause")]
#[only_in(guilds)]
#[aliases("pause", "resume")]
#[description("Toggles pause/resume for the current playback")]
#[num_args(0)]
async fn play_pause(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    _send_response(
        &msg.channel_id,
        &ctx.http,
        framework::music::play_pause(ctx, guild).await,
    )
    .await
}

#[command]
#[aliases("l", "verpiss_dich")]
#[only_in(guilds)]
#[description("Cleares the whole queue and disconnects the bot")]
#[num_args(0)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    _send_response(
        &msg.channel_id,
        &ctx.http,
        framework::music::leave(ctx, guild).await,
    )
    .await
}

#[command]
#[only_in(guilds)]
#[description("Mutes the bot for everyone")]
#[num_args(0)]
async fn mute(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    _send_response(
        &msg.channel_id,
        &ctx.http,
        framework::music::mute(ctx, guild).await,
    )
    .await
}

#[command]
#[only_in(guilds)]
#[aliases("p")]
#[description("Queues music/video sources from links")]
#[usage("[link]")]
#[example("play https://www.youtube.com/watch?v=vRpbtf8_7XM")]
#[num_args(1)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    // take the url from the message
    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            let mut e = default_embed();
            set_defaults_for_error(&mut e, "must provide a URL to a video or audio");
            return _send_response(&msg.channel_id, &ctx.http, e).await;
        }
    };
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;
    _send_response(
        &msg.channel_id,
        &ctx.http,
        framework::music::play(ctx, guild_id, url).await,
    )
    .await
}

#[command]
#[only_in(guilds)]
#[description("Skip the current song")]
#[num_args(0)]
#[aliases("s", "next", "fs")]
async fn skip(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    _send_response(
        &msg.channel_id,
        &ctx.http,
        framework::music::skip(ctx, guild).await,
    )
    .await
}

#[command]
#[only_in(guilds)]
#[description("Stops and cleares the current queue")]
#[num_args(0)]
async fn stop(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    _send_response(
        &msg.channel_id,
        &ctx.http,
        framework::music::stop(ctx, guild).await,
    )
    .await
}

#[command]
#[only_in(guilds)]
#[description("Undeafens the bot")]
#[num_args(0)]
async fn undeafen(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    _send_response(
        &msg.channel_id,
        &ctx.http,
        framework::music::undeafen(ctx, guild).await,
    )
    .await
}

#[command]
#[only_in(guilds)]
#[description("Unmutes the bot")]
#[num_args(0)]
async fn unmute(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    _send_response(
        &msg.channel_id,
        &ctx.http,
        framework::music::unmute(ctx, guild).await,
    )
    .await
}

#[command("loop")]
#[only_in(guilds)]
#[description("Loops the current song x times")]
#[num_args(1)]
async fn loop_song(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let times = match args.single::<usize>() {
        Ok(url) => url,
        Err(_) => {
            let mut e = default_embed();
            set_defaults_for_error(&mut e, "invalid loop counter");
            return _send_response(&msg.channel_id, &ctx.http, e).await;
        }
    };

    let guild = msg.guild(&ctx.cache).await.unwrap();
    _send_response(
        &msg.channel_id,
        &ctx.http,
        framework::music::loop_song(&ctx, guild.id, times).await,
    )
    .await
}

async fn _send_response(
    channel_id: &ChannelId,
    http: &Arc<Http>,
    embed: CreateEmbed,
) -> CommandResult {
    channel_id
        .send_message(http, |m| m.set_embed(embed))
        .await?;
    Ok(())
}
