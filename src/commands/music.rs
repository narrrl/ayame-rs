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
// use crate::model::youtube::*;

type Result<T> = std::result::Result<T, String>;

#[command]
#[only_in(guilds)]
#[description("Deafens the bot")]
#[num_args(0)]
async fn deafen(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    _send_response(
        &msg.channel_id,
        &ctx.http,
        framework::music::deafen(ctx, guild_id).await,
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
    _send_response(
        &msg.channel_id,
        &ctx.http,
        framework::music::join(&ctx, &guild, author_id).await,
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
    let guild = msg.guild(&ctx.cache).await.unwrap().id;
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
    let guild = msg.guild(&ctx.cache).await.unwrap().id;
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
    let guild = msg.guild(&ctx.cache).await.unwrap().id;
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
#[example("https://www.youtube.com/watch?v=vRpbtf8_7XM")]
#[num_args(1)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    // take the url from the message
    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            return _send_response(
                &msg.channel_id,
                &ctx.http,
                Err("must provide a URL to a video or audio".to_string()),
            )
            .await;
        }
    };
    let guild = msg.guild(&ctx.cache).await.unwrap();

    let manager = framework::music::_get_songbird(&ctx).await;
    if manager.get(guild.id).is_none() {
        let author_id = msg.author.id;
        let result = framework::music::join(ctx, &guild, author_id).await;
        let is_err = result.is_err().clone();
        let _ = _send_response(&msg.channel_id, &ctx.http, result).await;
        if is_err {
            return Ok(());
        }
    }
    _send_response(
        &msg.channel_id,
        &ctx.http,
        framework::music::play(ctx, &guild, &msg.channel_id, &msg.author.id, url).await,
    )
    .await
}

#[command]
#[only_in(guilds)]
#[description("Skip the current song")]
#[num_args(0)]
#[aliases("s", "next", "fs")]
async fn skip(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap().id;
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
    let guild = msg.guild(&ctx.cache).await.unwrap().id;
    _send_response(
        &msg.channel_id,
        &ctx.http,
        framework::music::stop(ctx, guild).await,
    )
    .await
}

// #[command("search")]
// #[only_in(guilds)]
// #[description("Searches for songs on youtube")]
// #[min_args(1)]
// async fn search(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
//     let guild_id = msg.guild_id.unwrap();
//     let guild = guild_id.to_guild_cached(&ctx.cache).await.unwrap();
//     let manager = framework::music::_get_songbird(&ctx).await;
//     let channel_id = msg.channel_id;
//
//     let conf = &crate::CONFIG;
//     let mut req = YoutubeSearch::new(&conf.youtube_api_key());
//     req.set_amount(5).set_filter(Type::VIDEO);
//
//     let search_term = args.message();
//
//     let res = match req.search(search_term).await {
//         Ok(res) => res,
//         Err(_) => {
//             let mut e = default_embed();
//             set_defaults_for_error(&mut e, "fatal error creating search");
//             _send_response(&channel_id, &ctx.http, Ok(e)).await;
//             return Ok(());
//         }
//     };
//
//     if res.results().is_empty() {
//         let mut e = default_embed();
//         set_defaults_for_error(&mut e, "noting found");
//         _send_response(&channel_id, &ctx.http, Ok(e)).await;
//         return Ok(());
//     }
//
//     let mut e = default_embed();
//     e.title("Select a result:");
//     e.description("Type the according number to select the song.");
//     for (i, result) in res.results().iter().enumerate() {
//         if i == 0 {
//             e.image(&result.thumbnail().url());
//         }
//         e.field(&format!("Index: {}", i), hyperlink_result(result), false);
//     }
//
//     _send_response(&channel_id, &ctx.http, Ok(e)).await;
//     let choice: &YoutubeResult = match &msg
//         .author
//         .await_reply(&ctx)
//         .timeout(std::time::Duration::from_secs(15))
//         .await
//     {
//         Some(answer) => {
//             let content = answer.content_safe(&ctx.cache).await;
//             if let Ok(index) = content.parse::<usize>() {
//                 if index < res.results().len() {
//                     // unwrap because index < len
//                     res.results().get(index).unwrap()
//                 } else {
//                     return Ok(());
//                 }
//             } else {
//                 return Ok(());
//             }
//         }
//         None => {
//             return Ok(());
//         }
//     };
//
//     // TODO: fix duplicated code
//     if manager.get(guild_id).is_none() {
//         let author_id = msg.author.id;
//         let result = framework::music::join(&ctx, &guild, author_id, channel_id).await;
//         let is_error = result.is_err();
//         _send_response(&channel_id, &ctx.http, result).await;
//         if is_error {
//             return Ok(());
//         }
//     }
//     let result = framework::music::play(&ctx, &guild, choice.url()).await;
//     _send_response(&channel_id, &ctx.http, result).await;
//     Ok(())
// }

async fn _send_response(
    channel_id: &ChannelId,
    http: &Arc<Http>,
    result: Result<CreateEmbed>,
) -> CommandResult {
    let embed = match result {
        Ok(embed) => embed,
        Err(why) => {
            let mut embed = default_embed();
            set_defaults_for_error(&mut embed, &why);
            embed
        }
    };
    channel_id
        .send_message(http, |m| m.set_embed(embed))
        .await?;
    Ok(())
}
