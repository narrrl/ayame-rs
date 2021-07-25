use crate::model::youtubedl::YTDL;
use crate::ShardManagerContainer;
use lazy_static::lazy_static;
use regex::Regex;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::Color;
use serenity::{
    client::bridge::gateway::ShardId,
    framework::standard::{macros::command, Args, CommandResult},
};
use tokio::task;

lazy_static! {
    pub static ref URL_REGEX: Regex = Regex::new(r"(http://www\.|https://www\.|http://|https://)?[a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,5}(:[0-9]{1,5})?(/.*)?").expect("Couldn't build URL Regex");
    pub static ref AUDIO_ONLY_REGEX: Regex = Regex::new(r"-audio").expect("Couldn't build URL Regex");
}

#[command("youtube-dl")]
#[bucket = "really_slow"]
#[aliases("ytd", "dl")]
async fn ytd(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if args.len() < 1 && args.len() > 2 {
        msg.reply(&ctx.http, "Please provide only one link to a video source")
            .await?;
        return Ok(());
    }

    let mut url = String::new();
    let mut audio_only = false;

    while !args.is_empty() {
        if let Ok(arg) = args.single::<String>() {
            if URL_REGEX.is_match(&arg) {
                url = arg;
            } else if AUDIO_ONLY_REGEX.is_match(&arg) {
                audio_only = true;
            }
        }
    }

    if url.is_empty() {
        msg.reply(&ctx.http, "Not a valid url").await?;
        return Ok(());
    }

    let id = msg.author.id.as_u64().clone();
    let channel_id = msg.channel_id.clone();
    let http = ctx.http.clone();

    task::spawn(async move {
        let mut ytdl = YTDL::new(channel_id, id, http);
        ytdl.set_defaults();
        if audio_only {
            ytdl.set_audio_only();
        }
        ytdl.start_download(url).await
    });
    Ok(())
}

#[command]
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

    match runner.latency {
        Some(latency) => {
            msg.reply(&ctx.http, &format!("Pong! {:?}", latency))
                .await?
        }
        None => msg.reply(&ctx.http, "Pong!").await?,
    };
    Ok(())
}

#[command]
async fn invite(ctx: &Context, msg: &Message) -> CommandResult {
    let current_user = ctx.http.get_current_user().await?;
    let application_id = current_user.id.as_u64();
    let invite_link = format!("https://discord.com/api/oauth2/authorize?client_id={}&permissions=8&scope=applications.commands%20bot", application_id);
    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title("Invite the bot to your server");
                e.url(invite_link);
                if let Some(url) = current_user.avatar_url() {
                    e.thumbnail(&url);
                }
                e.color(Color::from_rgb(238, 14, 97));
                e
            })
        })
        .await?;

    Ok(())
}

#[command]
async fn mock(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let channel_id = msg.channel_id;
    msg.delete(&ctx.http).await?;
    let msg = crate::model::mock_text(&args.message());

    channel_id
        .send_message(&ctx.http, |m| m.content(msg))
        .await?;
    Ok(())
}
