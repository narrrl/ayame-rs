use crate::framework;
use crate::ShardManagerContainer;
use lazy_static::lazy_static;
use regex::Regex;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::{
    client::bridge::gateway::ShardId,
    framework::standard::{macros::command, Args, CommandResult},
};

lazy_static! {
    pub static ref URL_REGEX: Regex = Regex::new(r"(http://www\.|https://www\.|http://|https://)?[a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,5}(:[0-9]{1,5})?(/.*)?").expect("Couldn't build URL Regex");
    pub static ref AUDIO_ONLY_REGEX: Regex = Regex::new(r"-audio").expect("Couldn't build URL Regex");
    pub static ref TIMESTAMP_REGEX: Regex = Regex::new(r"(\d{2}:)?(\d{2}:)?(\d+)(\.\d{2})?").expect("Couldn't build URL Regex");
}

#[command("youtube-dl")]
#[aliases("ytd", "dl")]
#[usage("(-audio) [link] (hh:mm:ss:ms) (hh:mm:ss:ms)")]
#[description("Download videos/audio from different sources")]
#[example("https://www.youtube.com/watch?v=dQw4w9WgXcQ 0 5")]
#[example("-audio https://www.youtube.com/watch?v=4Bw2GwAbPuQ 0 1.2")]
#[min_args(1)]
#[max_args(5)]
async fn ytd(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut url = String::new();
    let mut audio_only = false;
    let mut start = None;
    let mut end = None;

    while !args.is_empty() {
        if let Ok(arg) = args.single::<String>() {
            if URL_REGEX.is_match(&arg) {
                url = arg;
            } else if AUDIO_ONLY_REGEX.is_match(&arg) {
                audio_only = true;
            } else if TIMESTAMP_REGEX.is_match(&arg) {
                let stamp = match crate::model::Timestamp::from_string(&arg) {
                    Ok(stömp) => stömp,
                    Err(_why) => {
                        return Ok(());
                    }
                };

                if let Some(_) = start {
                    end = Some(stamp);
                } else {
                    start = Some(stamp);
                }
            } else {
                msg.reply(&ctx.http, &format!("Invalid input {}", &arg))
                    .await?;
                return Ok(());
            }
        } else {
            return Ok(());
        }
    }

    if url.is_empty() {
        msg.reply(&ctx.http, "Not a valid url").await?;
        return Ok(());
    }

    let id = msg.author.id.as_u64().clone();
    framework::ytd_with_stamps(&ctx.http, url, id, msg.channel_id, audio_only, start, end).await
}

#[command]
#[num_args(0)]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let shard_manager = match data.get::<ShardManagerContainer>() {
        Some(shard_manager) => shard_manager,
        None => {
            msg.reply(ctx, "There was a problem getting the shard manager")
                .await?;

            return Ok(());
        }
    };
    let manager = shard_manager.lock().await;
    let runners = manager.runners.lock().await;

    let runner = match runners.get(&ShardId(ctx.shard_id)) {
        Some(runner) => runner,
        None => {
            msg.reply(ctx, "No shard found").await?;

            return Ok(());
        }
    };
    framework::ping(&ctx.http, msg, &runner).await
}

#[command]
#[num_args(0)]
async fn invite(ctx: &Context, msg: &Message) -> CommandResult {
    framework::invite(&ctx.http, &msg.channel_id).await
}

#[command]
#[min_args(1)]
#[usage("[text...]")]
#[description("Converts your message to random upper and lower cases")]
async fn mock(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let text = args.message();
    framework::mock(&ctx.http, msg, text).await
}

#[command("guildicon")]
#[aliases("gi", "icon")]
#[only_in(guilds)]
#[num_args(0)]
async fn guild_icon(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.expect("Couldn't get guild");
    framework::guild_icon(&ctx.http, guild, msg).await
}

#[command("avatar")]
#[aliases("av", "pb")]
#[only_in(guilds)]
#[num_args(1)]
#[usage("[@user]")]
async fn avatar(ctx: &Context, msg: &Message) -> CommandResult {
    let user = match msg.mentions.get(msg.mentions.len() - 1) {
        Some(user) => user,
        None => {
            msg.reply(&ctx.http, "Invalid user specified").await?;
            return Ok(());
        }
    };
    framework::avatar(&ctx.http, msg, user).await
}
