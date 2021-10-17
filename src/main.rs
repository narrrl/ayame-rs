// commands
mod commands;
mod error;

// models like the youtube downloader
mod model;

// Config, etc ...
mod configuration;

mod framework;

use chrono::{offset::Local, Timelike};
use configuration::Config;
use serenity::{
    async_trait,
    model::{gateway::Activity, guild::Guild, id::GuildId, Permissions},
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

use std::sync::atomic::Ordering;
use std::{collections::HashSet, fs::remove_dir_all, sync::Arc, time::Duration};
use std::{path::PathBuf, sync::atomic::AtomicBool};

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
    is_loop_running: AtomicBool,
}

// Ready and Resumed events to notify if the bot has started/resumed
// TODO: this is the ugliest code i've ever written
#[async_trait]
impl EventHandler for Handler {
    async fn guild_create(&self, ctx: Context, guild: Guild, is_new: bool) {
        if !is_new {
            return;
        }
        info!("bot joined guild: {:?}, creating slash commands", guild.id);
        for cmd in get_all_create_commands(Scope::GUILD).iter() {
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

        let guilds = ctx.cache.guilds().await;
        let commands = get_all_create_commands(Scope::GUILD);
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
        }
        info!("Connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        info!("Cache built successfully!");
        let ctx = Arc::new(ctx);
        if !self.is_loop_running.load(Ordering::Relaxed) {
            let ctx_clone = Arc::clone(&ctx);
            tokio::spawn(async move {
                loop {
                    set_status_to_current_time(Arc::clone(&ctx_clone)).await;
                    let sleep_timer = (60 - Local::now().second()) as u64;
                    tokio::time::sleep(Duration::from_secs(sleep_timer)).await;
                }
            });

            // Now that the loop is running, we set the bool to true
            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
    }
}

async fn set_status_to_current_time(ctx: Arc<Context>) {
    let current_time = Local::now();
    let formatted_time = current_time.format("%H:%M, %a %b %e").to_string();

    ctx.set_activity(Activity::playing(&formatted_time)).await;
}

#[group]
#[commands(ping, ytd, invite, mock, guild_icon, avatar, guild_info)]
#[description = "A group with lots of different commands"]
struct General;

#[group]
#[commands(
    deafen,
    join,
    leave,
    mute,
    skip,
    stop,
    play,
    play_pause,
    now_playing,
    loop_song
)]
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
            is_loop_running: AtomicBool::new(false),
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
