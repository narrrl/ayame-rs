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
#[usage("(-audio) [link]")]
#[description("Download videos/audio from different sources")]
#[min_args(1)]
#[max_args(2)]
async fn ytd(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
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
#[num_args(0)]
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
#[min_args(1)]
#[usage("[text...]")]
#[description("Converts your message to random upper and lower cases")]
async fn mock(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let channel_id = msg.channel_id;
    msg.delete(&ctx.http).await?;
    let msg = crate::model::mock_text(&args.message());

    channel_id
        .send_message(&ctx.http, |m| m.content(msg))
        .await?;
    Ok(())
}

#[command("guildicon")]
#[aliases("gi", "icon")]
#[only_in(guilds)]
#[num_args(0)]
async fn guild_icon(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.expect("Couldn't get guild");
    let icon = match guild.icon_url() {
        Some(url) => url,
        None => {
            msg.reply(&ctx.http, "Guild has no icon").await?;
            return Ok(());
        }
    };
    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.image(icon);
                e.color(Color::from_rgb(238, 14, 97));
                e
            })
        })
        .await?;
    Ok(())
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

    let icon = match user.avatar_url() {
        Some(user) => user,
        None => {
            msg.reply(&ctx.http, "User has no avatar").await?;
            return Ok(());
        }
    };
    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.image(icon);
                e.color(Color::from_rgb(238, 14, 97));
                e
            })
        })
        .await?;

    Ok(())
}
