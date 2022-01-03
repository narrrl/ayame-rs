// commands
mod commands;
mod error;

// models like the youtube downloader
mod model;

// Config, etc ...
mod configuration;

mod framework;

mod database;

use configuration::Config;
use serenity::{
    async_trait,
    model::{
        guild::Guild,
        id::GuildId,
        prelude::{Activity, VoiceState},
        Permissions,
    },
};
use serenity::{
    client::bridge::gateway::{GatewayIntents, ShardManager},
    framework::{
        standard::{
            help_commands,
            macros::{group, help},
            Args, CommandGroup, CommandResult, HelpOptions,
        },
        StandardFramework,
    },
    http::Http,
    model::{
        channel::Message,
        event::ResumedEvent,
        gateway::Ready,
        id::UserId,
        interactions::{application_command::ApplicationCommand, Interaction},
    },
    prelude::*,
};
use songbird::SerenityInit;

use std::path::PathBuf;
use std::thread::sleep;
use std::{
    collections::{HashMap, HashSet},
    fs::remove_dir_all,
    sync::Arc,
    time::Duration,
};

use tracing::{error, info};

use commands::{admin::*, general::*, music::*, owner::*};

use framework::slash_commands::*;

use lazy_static::*;

use crate::model::discord_utils::check_msg;

pub const COLOR: &str = "#EE0E61";
pub const COLOR_ERROR: &str = "#CC0000";

lazy_static! {
    pub static ref CONFIG: Config = configuration::config();
    pub static ref BOT_DIR: PathBuf = {
        let mut dir = std::env::current_exe().expect("Couldn't get bot directory");
        dir.pop();
        dir
    };
    pub static ref NEEDED_PERMISSIONS: Permissions = {
        let mut perms = Permissions::default();
        perms.toggle(Permissions::MANAGE_EMOJIS);
        perms.toggle(Permissions::READ_MESSAGES);
        perms.toggle(Permissions::SEND_MESSAGES);
        perms.toggle(Permissions::MANAGE_MESSAGES);
        perms.toggle(Permissions::EMBED_LINKS);
        perms.toggle(Permissions::ATTACH_FILES);
        perms.toggle(Permissions::READ_MESSAGE_HISTORY);
        perms.toggle(Permissions::USE_EXTERNAL_EMOJIS);
        perms.toggle(Permissions::CONNECT);
        perms.toggle(Permissions::SPEAK);
        perms.toggle(Permissions::MUTE_MEMBERS);
        perms.toggle(Permissions::DEAFEN_MEMBERS);
        perms.toggle(Permissions::MOVE_MEMBERS);
        perms.toggle(Permissions::USE_VAD);
        perms.toggle(Permissions::USE_PUBLIC_THREADS);
        perms.toggle(Permissions::USE_PRIVATE_THREADS);
        perms.toggle(Permissions::CREATE_INVITE);
        perms
    };
}

pub struct ShardManagerContainer;

// shard manager
impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct Handler {
    disconnects: Arc<Mutex<HashMap<u64, bool>>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn voice_state_update(
        &self,
        ctx: Context,
        guild: Option<GuildId>,
        _old: Option<VoiceState>,
        _new: VoiceState,
    ) {
        let guild_id = match guild {
            Some(id) => id,
            None => return,
        };
        // we check if a disconnect event is already runnig
        let mut disconnects = self.disconnects.lock().await;
        match disconnects.get(&guild_id.0) {
            // if yes, just return and ignore
            // for example if multiple users are disconnecting
            // like for example when the fun gaming eve has ended :(
            Some(b) if *b => return,
            // else if the guild_id is not present or if no disconnect event is running insert true
            // and start the disconnect event
            _ => disconnects.insert(guild_id.0, true),
        };
        // we drop the mutex guard to move it to another thread
        drop(disconnects);
        // we get the arc mutex and clone it, beause it gets borrowed by the async move
        let disconnects = self.disconnects.clone();
        tokio::spawn(async move {
            // we wait, maybe they just want to switch channel
            sleep(Duration::from_secs(20));
            // we can insert false, most likely everyone lefted already and the bot is about to
            // disconnect
            disconnects.lock().await.insert(guild_id.0, false);
            let manager = songbird::get(&ctx)
                .await
                .expect("Couldn't get songbird manager");
            let handle = match manager.get(guild_id) {
                Some(handle) => handle,
                None => return,
            };
            let call = handle.lock().await;
            let mut users = vec![];
            if let Some(current_channel) = call.current_channel() {
                let channel = match ctx.cache.guild_channel(current_channel.0).await {
                    Some(channel) => channel,
                    None => return,
                };
                users = match channel.members(&ctx.cache).await {
                    Ok(users) => users,
                    Err(_) => return,
                };
            }
            // we can drop the call, because we have everything we need
            drop(call);
            let mut no_user_connected = true;
            for user in users.iter() {
                if !user.user.bot {
                    no_user_connected = false;
                    break;
                }
            }
            if no_user_connected {
                if let Err(e) = manager.remove(guild_id).await {
                    error!("failed to remove songbird manager: {:?}", e);
                }
            }
        });
    }
    async fn guild_create(&self, ctx: Context, guild: Guild, is_new: bool) {
        if !is_new {
            return;
        }
        info!("bot joined guild: {:?}, creating slash commands", guild.id);
        ctx.set_activity(Activity::playing(&format!(
            "in {} guilds!",
            ctx.cache.guilds().await.len()
        )))
        .await;
        for cmd in get_slash_handler()
            .get_all_create_commands(Scope::GUILD)
            .iter()
        {
            check_msg(
                guild
                    .id
                    .create_application_command(&ctx.http, |command| {
                        command.clone_from(&cmd);
                        command
                    })
                    .await,
            );
        }
    }
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let key = command.data.name.as_str().to_string();
            let handler = get_slash_handler();
            if let Some(slash) = handler.find(&key) {
                if let Err(why) = slash.run(ctx, command).await {
                    error!("failed: {:?}", why);
                }
            }
        }
    }
    async fn ready(&self, ctx: Context, ready: Ready) {
        check_msg(
            ApplicationCommand::set_global_application_commands(&ctx.http, |commands| {
                create_commands(commands)
            })
            .await,
        );

        let handler = get_slash_handler();
        info!(
            "Found guild commands: {:?}",
            handler
                .get_all_aliases(Scope::GLOBAL)
                .iter()
                .map(|s| &**s)
                .collect::<Vec<&str>>()
                .join(", ")
        );

        let guilds = ctx.cache.guilds().await;
        let commands = handler.get_all_create_commands(Scope::GUILD);
        info!(
            "Found guild commands: {:?}",
            handler
                .get_all_aliases(Scope::GUILD)
                .iter()
                .map(|s| &**s)
                .collect::<Vec<&str>>()
                .join(", ")
        );
        for id in guilds.iter() {
            let id = id.clone();
            let http = ctx.http.clone();
            let all_cmd = commands.clone();
            info!("creating application commands for guild: {:?}", &id);
            tokio::spawn(async move {
                for cmd in all_cmd.iter() {
                    check_msg(
                        id.create_application_command(&http, |command| {
                            command.clone_from(&cmd);
                            command
                        })
                        .await,
                    );
                }
            });
            if let Ok(guild_commands) = id.get_application_commands(&ctx.http).await {
                if commands.len() != guild_commands.len() {
                    info!(
                        "found dublicated commands for guild {:?} -> deleting dubicates",
                        id
                    );
                    let to_remove = handler.get_all_aliases(Scope::GLOBAL);
                    guild_commands
                        .iter()
                        .filter(|c| to_remove.iter().any(|n| n.eq(&c.name)))
                        .for_each(|c| {
                            let cmd_id = c.id.clone();
                            let http = ctx.http.clone();
                            tokio::spawn(async move {
                                check_msg(id.delete_application_command(&http, cmd_id).await);
                            });
                        });
                }
            }
        }

        info!("Connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
    async fn cache_ready(&self, ctx: Context, guilds: Vec<GuildId>) {
        info!("Cache built successfully!");
        ctx.set_activity(Activity::playing(&format!("in {} guilds!", guilds.len())))
            .await;
    }
}

#[group]
#[commands(ping, ytd, invite, mock, guild_icon, avatar, guild_info)]
#[description = "A group with lots of different commands"]
struct General;

#[group]
#[commands(deafen, join, leave, mute, skip, stop, play, play_pause, now_playing)]
struct Music;

#[group]
#[commands(shutdown)]
#[description = "Bot-owner commands"]
struct Owner;

#[group]
#[commands(addemote)]
#[description = "A group for admin utility to manage your server"]
struct Admin;

#[help]
#[individual_command_tip = "Hewwo! こんにちは！안녕하세요~\n\n\
If you want more information about a specific command, just pass the command as argument."]
#[command_not_found_text = "Could not find: `{}`."]
#[suggestion_text("Did you mean `{}`?")]
#[embed_success_colour = "#EE0E61"]
#[embed_error_colour = "#CC0000"]
#[max_levenshtein_distance(3)]
#[lacking_permissions = "Hide"]
#[lacking_role = "Nothing"]
#[wrong_channel = "Strike"]
async fn help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

pub fn get_file(name: &str) -> PathBuf {
    let mut dir = BOT_DIR.clone();
    dir.push(name);
    dir
}

#[tokio::main]
async fn main() {
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "INFO");
    }
    tracing_subscriber::fmt::init();

    let token = CONFIG.token();

    let http = Http::new_with_token(&token);

    // get owners and bot id from application
    let (owners, bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            if let Some(team) = info.team {
                owners.insert(team.owner_user_id);
            } else {
                owners.insert(info.owner.id);
            }

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Create bot
    //load bot prefix from config
    let prefix: &str = &CONFIG.prefix();

    info!("Cleaning temporary directory");
    let _ = remove_dir_all(get_file("tmp"));

    let framework = StandardFramework::new()
        .configure(|c| {
            c.owners(owners)
                .prefix(prefix)
                .on_mention(Some(bot_id))
                .with_whitespace(true)
                .delimiters(vec![" "])
                .no_dm_prefix(true)
        })
        .group(&GENERAL_GROUP)
        .group(&OWNER_GROUP)
        .group(&ADMIN_GROUP)
        .group(&MUSIC_GROUP)
        .help(&HELP)
        .bucket("youtubedl", |b| b.time_span(180).limit(1))
        .await;
    let application_id: u64 = CONFIG.get_application_id();

    let mut client = Client::builder(&token)
        .framework(framework)
        .intents(GatewayIntents::all())
        .event_handler(Handler {
            disconnects: Arc::new(Mutex::new(HashMap::new())),
        })
        .application_id(application_id)
        .register_songbird()
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
    }

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
