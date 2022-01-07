pub mod admin;
pub mod general;
pub mod music;
pub mod owner;
pub mod slash_commands;

use std::sync::Arc;

use crate::model::discord_utils::{check_msg, default_embed};
use crate::model::youtubedl::YTDL;
use crate::model::Timestamp;
use lazy_static::lazy_static;
use regex::Regex;
use serenity::client::bridge::gateway::ShardRunnerInfo;
use serenity::client::Cache;
use serenity::framework::standard::CommandResult;
use serenity::http::Http;
use serenity::model::prelude::*;
use tokio::task;

lazy_static! {
    pub static ref URL_REGEX: Regex = Regex::new(r"(http://www\.|https://www\.|http://|https://)?[a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,5}(:[0-9]{1,5})?(/.*)?").expect("Couldn't build URL Regex");
    pub static ref AUDIO_ONLY_REGEX: Regex = Regex::new(r"-audio").expect("Couldn't build URL Regex");
}

pub async fn ytd_with_stamps(
    http: &Arc<Http>,
    url: String,
    author_id: u64,
    channel_id: ChannelId,
    audio_only: bool,
    start: Option<Timestamp>,
    end: Option<Timestamp>,
    cache: &Arc<Cache>,
) -> CommandResult {
    let http = http.clone();
    let cache = cache.clone();
    task::spawn(async move {
        let mut ytdl = YTDL::new(channel_id, author_id, http, cache);
        ytdl.set_defaults();
        if let Some(start) = start {
            ytdl.set_start(start);
        }
        if let Some(end) = end {
            ytdl.set_end(end);
        }
        if audio_only {
            ytdl.set_audio_only();
        }
        ytdl.start_download(url).await
    });
    Ok(())
}

pub async fn ping(http: &Arc<Http>, msg: &Message, runner: &ShardRunnerInfo) -> CommandResult {
    match runner.latency {
        Some(latency) => msg.reply(http, &format!("Pong! {:?}", latency)).await?,
        None => msg.reply(http, "Pong!").await?,
    };
    Ok(())
}

pub async fn guild_icon(http: &Arc<Http>, guild: Guild, msg: &Message) -> CommandResult {
    let icon = match guild.icon_url() {
        Some(url) => url,
        None => {
            msg.reply(http, "Guild has no icon").await?;
            return Ok(());
        }
    };
    let mut e = default_embed();
    e.image(icon);
    check_msg(msg.channel_id.send_message(http, |m| m.set_embed(e)).await);
    Ok(())
}

pub async fn avatar(http: &Arc<Http>, msg: &Message, user: &User) -> CommandResult {
    let icon = match user.avatar_url() {
        Some(user) => user,
        None => {
            msg.reply(http, "User has no avatar").await?;
            return Ok(());
        }
    };
    let mut e = default_embed();
    e.image(icon);
    check_msg(msg.channel_id.send_message(http, |m| m.set_embed(e)).await);

    Ok(())
}

pub async fn guild_info(http: &Arc<Http>, guild: Guild, msg: &Message) -> CommandResult {
    let icon = match guild.icon_url() {
        Some(url) => {
            let mut url = url.to_string();
            url.push_str("?size=512");
            url
        }
        None => String::new(),
    };
    let creation_date = guild.id.created_at();

    let mut message = format!(
        "
        Name: {}
        Created: {}
        Owner: <@!{}>
        Members: {}
        Bot joined: {}
        ",
        guild.name,
        creation_date.format("%H:%M, %a %b %e %Y").to_string(),
        guild.owner_id.as_u64(),
        // admins,
        match guild.max_members {
            Some(max) => format!("{}/{}", guild.member_count, max),
            None => guild.member_count.to_string(),
        },
        guild.joined_at.format("%H:%M, %a %b %e %Y").to_string()
    );

    if let Some(ch) = guild.afk_channel_id {
        message.push_str(&format!("AFK-Channel: <#{}>", ch.as_u64()));
    }

    let mut e = default_embed();
    e.image(icon).title(guild.name).description(message);

    check_msg(msg.channel_id.send_message(http, |m| m.set_embed(e)).await);

    Ok(())
}
