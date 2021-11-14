use crate::model::discord_utils;
use serenity::builder::CreateEmbed;
use serenity::http::Http;
use serenity::model::prelude::*;
use std::sync::Arc;

// TODO: write helper?
pub async fn invite(http: &Arc<Http>) -> CreateEmbed {
    let mut e = discord_utils::default_embed();
    // get the current connected user
    let current_user = http
        .get_current_user()
        .await
        .expect("Couldn't get current_user");
    // get the total amount of guilds the bot joined (flex dies das)
    let guild_amount = match current_user.guilds(&http).await {
        Ok(guilds) => guilds.len(),
        Err(_) => 0,
    };
    // get the needed permissions from the main crate
    let needed_perms = crate::NEEDED_PERMISSIONS.clone();
    // create the invite url with the oauth2 scope for slash commands
    let invite_url = current_user
        .invite_url_with_oauth2_scopes(
            http,
            needed_perms,
            &[OAuth2Scope::ApplicationsCommands, OAuth2Scope::Bot],
        )
        .await
        .expect("Couldn't create invite url");
    // same but with no brain admin perms
    let admin_invite = current_user
        .invite_url_with_oauth2_scopes(
            http,
            Permissions::ADMINISTRATOR,
            &[OAuth2Scope::ApplicationsCommands, OAuth2Scope::Bot],
        )
        .await
        .expect("Couldn't create invite url");
    e.title("Invite the Bot!");
    // set the title url to be the admin url (because people who are used to click anything in the
    // internet are likely to complain about the bot suddenly breaking, when new features get
    // introduced
    e.url(&admin_invite);
    // nice touch to set the avatar as thumbnail
    if let Some(url) = current_user.avatar_url() {
        e.thumbnail(&url);
    }
    // footer with the total joined guilds
    e.footer(|f| {
        if let Some(url) = current_user.avatar_url() {
            f.icon_url(&url);
        }
        f.text(&format!("Joined Guilds: {}", guild_amount));
        f
    });
    // author is obv the bot
    e.author(|a| {
        if let Some(url) = current_user.avatar_url() {
            a.icon_url(&url);
        }
        a.name(&current_user.name);
        a
    });
    // long text which will never be read by anyone.
    e.description(
        "Those are the requirements for the bot to run without any restrictions. \
        The bot is open source and the source code can be found on \
        [Github](https://github.com/nirusu99/ayame-rs). 

        [Click here](invite_url) to invite the bot with required permissions
        [Click here](admin_invite) to invite the bot with administrator privileges

        Why should you invite the bot with elevated permissions? 
        Because future releases might not be supported by the current set of permissions. \
        That means if a new command or feature doesn't work then that could mean that you have to \
        invite the bot again with the new set of permissions. \
        Administrator removes any restrictions which means the bot won't break (atleast never \
        because of permissions :D)."
            .replace("invite_url", &invite_url)
            .replace("admin_invite", &admin_invite),
    );
    e.field(
        "Required Permissions:",
        needed_perms.to_string().replace(", ", "\n"),
        false,
    );
    e.field(
        "Required OAuth2Scopes:",
        "ApplicationsCommands (Slash Commands)
        Bot (Well I guess)",
        false,
    );
    // return embed
    e
}
