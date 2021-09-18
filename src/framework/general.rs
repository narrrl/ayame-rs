use crate::model::discord_utils;
use serenity::builder::CreateEmbed;
use serenity::http::Http;
use serenity::model::prelude::*;
use std::sync::Arc;
use tracing::error;

pub async fn invite(http: &Arc<Http>) -> CreateEmbed {
    let mut e = discord_utils::default_embed();
    let current_user = http
        .get_current_user()
        .await
        .expect("Couldn't get current_user");
    let guild_amount = match current_user.guilds(&http).await {
        Ok(guilds) => guilds.len(),
        Err(_) => 0,
    };
    let invite_url = current_user
        .invite_url_with_oauth2_scopes(
            http,
            Permissions::ADMINISTRATOR,
            &[OAuth2Scope::ApplicationsCommands, OAuth2Scope::Bot],
        )
        .await
        .expect("Couldn't create invite url");
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
        "Those are the requirements for the bot to run without any restrictions. \
        The bot is open source and the source code can be found on \
        [Github](https://github.com/nirusu99/nirust). 

        [Click here](invite_url) to get the bot to join your server"
            .replace("invite_url", &invite_url),
    );
    e.field("Required Permissions:", "- ADMINISTRATOR", true);
    e.field(
        "Required OAuth2Scopes:",
        "- ApplicationsCommands (Slash Commands)
        - Bot (Well I guess)",
        true,
    );
    e
}
