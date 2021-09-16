use std::sync::Arc;

use crate::model::youtubedl::YTDL;
use crate::model::Timestamp;
use lazy_static::lazy_static;
use regex::Regex;
use serenity::client::bridge::gateway::ShardRunnerInfo;
use serenity::framework::standard::CommandResult;
use serenity::http::Http;
use serenity::model::prelude::*;
use serenity::utils::Color;
use tokio::task;
use tracing::{debug, error, info};

lazy_static! {
    pub static ref URL_REGEX: Regex = Regex::new(r"(http://www\.|https://www\.|http://|https://)?[a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,5}(:[0-9]{1,5})?(/.*)?").expect("Couldn't build URL Regex");
    pub static ref AUDIO_ONLY_REGEX: Regex = Regex::new(r"-audio").expect("Couldn't build URL Regex");
}

//TODO: look for a way to merge slash and normal. Currently slash commands need to answer with
// a message. It is not enough to just send a message to the same channel.
//
// One idea would be to look into ['serenity::model::prelude::Context'] and try to merge the
// Context from slash and normal commands.

pub async fn ytd_with_stamps(
    http: &Arc<Http>,
    url: String,
    author_id: u64,
    channel_id: ChannelId,
    audio_only: bool,
    start: Option<Timestamp>,
    end: Option<Timestamp>,
) -> CommandResult {
    let http = http.clone();
    task::spawn(async move {
        let mut ytdl = YTDL::new(channel_id, author_id, http);
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

//TODO: make timestapms with slash work
#[allow(dead_code)]
pub async fn ytd(
    http: &Arc<Http>,
    url: String,
    author_id: u64,
    channel_id: ChannelId,
    audio_only: bool,
) -> CommandResult {
    let http = http.clone();
    task::spawn(async move {
        let mut ytdl = YTDL::new(channel_id, author_id, http);
        ytdl.set_defaults();
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

pub async fn invite(http: &Arc<Http>, channel_id: &ChannelId) -> CommandResult {
    let current_user = http.get_current_user().await?;
    let guild_amount = current_user.guilds(&http).await?.len();
    let invite_url = current_user
        .invite_url_with_oauth2_scopes(
            http,
            Permissions::ADMINISTRATOR,
            &[OAuth2Scope::ApplicationsCommands, OAuth2Scope::Bot],
        )
        .await?;
    channel_id
        .send_message(&http, |m| {
            m.embed(|e| {
                e.title("Invite the Bot!");
                e.url(&invite_url);
                if let Some(url) = current_user.avatar_url() {
                    e.thumbnail(&url);
                }
                e.footer(|f| {
                    if let Some(url) = current_user.avatar_url() {
                        f.icon_url(&url);
                    }
                    f.text(&format!("Joined Guilds: {}", guild_amount));
                    f
                });
                e.author(|a| {
                    if let Some(url) = current_user.avatar_url() {
                        a.icon_url(&url);
                    }
                    a.name(&current_user.name);
                    a
                });
                e.description(
                    "Those are the requirements for the bot to run without any restrictions.
                    **Required [Permissions]\
                    (https://discord.com/developers/docs/topics/permissions#permissions-bitwise-permission-flags)**:
                    - ADMINISTRATOR 

                    **Required [OAuth2Scopes]\
                    (https://discord.com/developers/docs/topics/oauth2#shared-resources-oauth2-scopes)**:
                    - ApplicationsCommands (Slash Commands)
                    - Bot (Well I guess)

                    The bot is open source and the source code can be found on \
                    [Github](https://github.com/nirusu99/nirust). 

                    [Click here](invite_url) to get the bot to join your server
                "
                    .replace("invite_url", &invite_url),
                );
                e.color(Color::from_rgb(238, 14, 97));
                e
            })
        })
        .await?;

    Ok(())
}

pub async fn mock(http: &Arc<Http>, msg: &Message, text: &str) -> CommandResult {
    let channel_id = msg.channel_id;
    msg.delete(http).await?;
    let msg = crate::model::mock_text(text);

    channel_id.send_message(http, |m| m.content(msg)).await?;
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
    msg.channel_id
        .send_message(http, |m| {
            m.embed(|e| {
                e.image(icon);
                e.color(Color::from_rgb(238, 14, 97));
                e
            })
        })
        .await?;
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
    msg.channel_id
        .send_message(http, |m| {
            m.embed(|e| {
                e.image(icon);
                e.color(Color::from_rgb(238, 14, 97));
                e
            })
        })
        .await?;

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

    let members = &guild.members;

    let mut admins = vec![];
    for (id, member) in members.iter() {
        if id.eq(&guild.owner_id) {
            continue;
        }
        if let Ok(perms) = guild.member_permissions(http, id).await {
            if !member.user.bot && perms.contains(Permissions::ADMINISTRATOR) {
                admins.push(id);
            }
        }
    }

    let admins = admins
        .into_iter()
        .map(|i| format!("<@!{}>", i.to_string()))
        .collect::<String>();

    let mut message = format!(
        "
        Name: {}
        Created: {}
        Owner: <@!{}>
        Admins: {}
        Members: {}
        Bot joined: {}
        ",
        guild.name,
        creation_date.format("%H:%M, %a %b %e %Y").to_string(),
        guild.owner_id.as_u64(),
        admins,
        match guild.max_members {
            Some(max) => format!("{}/{}", guild.member_count, max),
            None => guild.member_count.to_string(),
        },
        guild.joined_at.format("%H:%M, %a %b %e %Y").to_string()
    );

    if let Some(ch) = guild.afk_channel_id {
        message.push_str(&format!("AFK-Channel: <#{}>", ch.as_u64()));
    }

    msg.channel_id
        .send_message(http, |m| {
            m.embed(|e| {
                e.image(icon);
                e.color(Color::from_rgb(238, 14, 97));
                e.title(guild.name);
                e.description(message);
                e
            })
        })
        .await?;

    Ok(())
}
