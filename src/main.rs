// commands
mod commands;

// models like the youtube downloader
mod model;

// Config, etc ...
mod configuration;

mod framework;

use chrono::{offset::Local, Timelike};
use configuration::Config;
use model::youtubedl::YTDL;
use serenity::{
    async_trait,
    model::{gateway::Activity, id::GuildId},
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
        interactions::{
            application_command::{
                ApplicationCommand, ApplicationCommandInteractionDataOptionValue,
                ApplicationCommandOptionType,
            },
            Interaction, InteractionResponseType,
        },
    },
    prelude::*,
};
use tokio::task;

use std::sync::atomic::Ordering;
use std::{collections::HashSet, fs::remove_dir_all, sync::Arc, time::Duration};
use std::{path::PathBuf, sync::atomic::AtomicBool};

use tracing::{error, info};

use commands::{admin::*, general::*, owner::*};

use lazy_static::*;

lazy_static! {
    pub static ref CONFIG: Config = configuration::config();
    pub static ref BOT_DIR: PathBuf = {
        let mut dir = std::env::current_exe().expect("Couldn't get bot directory");
        dir.pop();
        dir
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
#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            match command.data.name.as_str() {
                "youtubedl" => {
                    let options = command
                        .data
                        .options
                        .get(0)
                        .expect("Expected user option")
                        .resolved
                        .as_ref()
                        .expect("Expected String object");
                    let audio_only = match command.data.options.get(1) {
                        Some(value) => {
                            let value = value.resolved.as_ref().expect("Expected bool object");
                            match value {
                                ApplicationCommandInteractionDataOptionValue::Boolean(b) => {
                                    b.clone()
                                }
                                _ => false,
                            }
                        }
                        None => false,
                    };
                    if let ApplicationCommandInteractionDataOptionValue::String(url) = options {
                        if crate::commands::general::URL_REGEX.is_match(url) {
                            let id = command.user.id.as_u64().clone();
                            let channel_id = command.channel_id.clone();
                            let http = ctx.http.clone();
                            let audio_only = audio_only;
                            let url = url.to_string();
                            let _ = command
                                .create_interaction_response(&ctx.http, |response| {
                                    response
                                        .kind(InteractionResponseType::ChannelMessageWithSource)
                                        .interaction_response_data(|message| {
                                            message.content("Revieved Event")
                                        })
                                })
                                .await;

                            task::spawn(async move {
                                let mut ytdl = YTDL::new(channel_id, id, http);
                                ytdl.set_defaults();
                                if audio_only {
                                    ytdl.set_audio_only();
                                }
                                if let Ok(message) =
                                    command.get_interaction_response(&ctx.http).await
                                {
                                    ytdl.set_update_message(&message);
                                }
                                ytdl.start_download(url.to_string()).await
                            });
                        } else {
                            let _ = command
                                .create_interaction_response(&ctx.http, |response| {
                                    response
                                        .kind(InteractionResponseType::ChannelMessageWithSource)
                                        .interaction_response_data(|message| {
                                            message.content("Invalid URL")
                                        })
                                })
                                .await;
                        }
                    }
                }
                "invite" => {
                    let _ = command
                        .create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|message| message.content("✅"))
                        })
                        .await;
                    let channel_id = command.channel_id;
                    let _ = framework::invite(&ctx.http, &channel_id).await;
                }
                "mock" => {
                    let _ = command
                        .create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|message| message.content("✅"))
                        })
                        .await;
                }
                _ => return (),
            };
        }
    }
    async fn ready(&self, ctx: Context, ready: Ready) {
        let commands = ApplicationCommand::set_global_application_commands(&ctx.http, |commands| {
            commands
                .create_application_command(|command| {
                    command
                        .name("youtubedl")
                        .description("Download Videos from a lot of sites")
                        .create_option(|option| {
                            option
                                .name("link")
                                .description("A link to a video source")
                                .kind(ApplicationCommandOptionType::String)
                                .required(true)
                        })
                        .create_option(|option| {
                            option
                                .name("audio_only")
                                .description("Download the video as mp3")
                                .kind(ApplicationCommandOptionType::Boolean)
                                .required(false)
                        })
                })
                .create_application_command(|command| {
                    command
                        .name("invite")
                        .description("Invite the bot to your server")
                })
                .create_application_command(|command| {
                    command
                        .name("mock")
                        .description("Converts your message to random upper and lower case")
                        .create_option(|option| {
                            option
                                .name("message")
                                .description("Your message that gets converted")
                                .kind(ApplicationCommandOptionType::String)
                                .required(true)
                        })
                })
        })
        .await;

        println!(
            "I now have the following global slash commands {:#?}",
            commands
        );
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
#[commands(ping, ytd, invite, mock, guild_icon, avatar)]
#[description = "A group with lots of different commands"]
struct General;

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
        .help(&HELP)
        // annote command with #[bucket = "really_slow"]
        // to limit command usage to 1 uses per 10 minutes
        .bucket("really_slow", |b| b.time_span(600).limit(1))
        .await;
    let application_id: u64 = CONFIG.get_application_id();

    let mut client = Client::builder(&token)
        .framework(framework)
        .intents(GatewayIntents::all())
        .event_handler(Handler {
            is_loop_running: AtomicBool::new(false),
        })
        .application_id(application_id)
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
