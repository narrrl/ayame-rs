use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use commands::general::*;
use commands::manage::*;
use commands::music::*;
use commands::owner::*;
use poise::serenity_prelude::UserId;
use poise::serenity_prelude::{self as serenity, Mutex};
use songbird::Songbird;
use songbird::SongbirdKey;
use tracing::{error, info};
use uuid::Uuid;

mod commands;
mod configuration;
mod error;
mod menu;
mod model;
mod music;
mod utils;
mod voice;
mod youtube;

pub const DEFAULT_DATABASE_URL: &str = "sqlite:database/database.sqlite";

#[derive(Clone)]
pub struct Data {
    // config can only be read, can't change
    config: Arc<configuration::Config>,
    // all uuid of songs mapped to the user that requested it
    song_queues: Arc<Mutex<HashMap<Uuid, UserId>>>,
    // all now_playing messages
    playing_messages: Arc<Mutex<HashMap<serenity::GuildId, serenity::MessageId>>>,
    // database
    database: sqlx::SqlitePool,
}
pub type Error = error::AyameError;

pub type Context<'a> = poise::Context<'a, Data, Error>;

async fn event_listener(
    ctx: &serenity::Context,
    event: &poise::Event<'_>,
    _framework: &poise::Framework<Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        poise::Event::Ready { data_about_bot } => {
            info!("{} is connected!", data_about_bot.user.name)
        }
        poise::Event::VoiceStateUpdate {
            old: Some(state),
            new,
        } => {
            let channel_id = match state.channel_id {
                Some(channel_id) => channel_id,
                _ => return Ok(()),
            };
            let channel = ctx.http.get_channel(channel_id.into()).await?;
            let guild_channel = match channel.guild() {
                Some(guild_channel) => guild_channel,
                _ => return Ok(()),
            };
            let current_user = ctx.http.get_current_user().await?;
            if is_bot_alone(&guild_channel, ctx, &current_user.id).await? {
                let songbird = music::get_serenity(ctx).await?;
                match new.channel_id {
                    Some(new_id) => {
                        music::join::join_serenity(&ctx, data, &guild_channel.guild_id, &new_id)
                            .await?;
                    }
                    None => music::leave::leave(&songbird, guild_channel.guild_id).await?,
                };
            }
        }
        _ => {}
    }

    Ok(())
}

async fn is_bot_alone(
    channel: &serenity::GuildChannel,
    ctx: &serenity::Context,
    bot_id: &serenity::UserId,
) -> Result<bool, Error> {
    let members = channel.members(&ctx.cache).await?;
    Ok(!members.iter().map(|m| &m.user).any(|u| !u.bot)
        && members.iter().any(|m| m.user.id == *bot_id))
}

#[poise::command(prefix_command, slash_command, ephemeral)]
async fn help(
    ctx: Context<'_>,
    #[description = "Command to display specific information about"] command: Option<String>,
) -> Result<(), Error> {
    let config = poise::builtins::HelpConfiguration {
        extra_text_at_bottom: "\
If you want more information about a specific command, just pass the command as argument.",
        ..Default::default()
    };

    poise::builtins::help(ctx, command.as_deref(), config).await?;

    Ok(())
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Command { error, ctx } => {
            if let Error::Input(_) = error {
                error.send_error(&ctx).await
            } else {
                error!("{:?}", error)
            }
        }
        poise::FrameworkError::CommandCheckFailed { error, ctx } => {
            if let Some(why) = error {
                why.send_error(&ctx).await
            } else {
                Error::Input("checks for the command failed")
                    .send_error(&ctx)
                    .await
            }
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                error!("Error while handling error: {}", e)
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "INFO");
    }
    let database_url = std::env::var("DATABASE_URL").unwrap_or(DEFAULT_DATABASE_URL.to_string());
    tracing_subscriber::fmt::init();
    let database = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            database_url
                .parse::<sqlx::sqlite::SqliteConnectOptions>()?
                .create_if_missing(true),
        )
        .await?;
    sqlx::migrate!("./migrations").run(&database).await?;

    Ok(run_discord_client(database).await?)
}

async fn run_discord_client(database: sqlx::SqlitePool) -> Result<(), anyhow::Error> {
    let config = configuration::config();
    let client = poise::Framework::build()
        .client_settings(move |client_builder: serenity::ClientBuilder| {
            // get songbird instance
            let voice = Songbird::serenity();
            client_builder
                .intents(serenity::GatewayIntents::all())
                // register songbird as VoiceGatewayManager
                .voice_manager_arc(voice.clone())
                // insert songbird into data
                .type_map_insert::<SongbirdKey>(voice)
        })
        .token(config.token())
        .options(get_discord_configuration(&config))
        .user_data_setup(|ctx, _data_about_bot, framework| {
            Box::pin(async move {
                // set activity to "{prefix}help"
                ctx.set_activity(serenity::Activity::listening(format!(
                    "{}help",
                    config.prefix()
                )))
                .await;
                let shard_manager = framework.shard_manager();
                tokio::spawn(async move {
                    tokio::signal::ctrl_c()
                        .await
                        .expect("Could not register ctrl+c handler");
                    shard_manager.lock().await.shutdown_all().await;
                });
                let shard_manager = framework.shard_manager();
                tokio::spawn(async move {
                    let term = Arc::new(AtomicBool::new(false));

                    if let Err(why) =
                        signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))
                    {
                        error!("couldn't register sigterm hook {:?}", why);
                        return;
                    }
                    while !term.load(Ordering::Relaxed) {
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                    shard_manager.lock().await.shutdown_all().await;
                });
                // create our data
                Ok(Data {
                    config: Arc::new(config),
                    song_queues: Arc::new(Mutex::new(HashMap::new())),
                    playing_messages: Arc::new(Mutex::new(HashMap::new())),
                    database,
                })
            })
        });

    Ok(client.run_autosharded().await?)
}

fn get_discord_configuration(
    config: &configuration::Config,
) -> poise::FrameworkOptions<Data, Error> {
    poise::FrameworkOptions {
        commands: vec![
            avatar(),
            help(),
            mock(),
            mockify(),
            uwu(),
            uwuify(),
            register(),
            unregister(),
            bind(),
            ping_bind(),
            mensa(),
            invite(),
            join(),
            leave(),
            play(),
            now_playing(),
            search(),
            skip(),
            shutdown(),
            addemote(),
            play_message_content(),
        ],
        listener: |ctx, event, framework, user_data| {
            Box::pin(event_listener(ctx, event, framework, user_data))
        },
        on_error: |error| Box::pin(on_error(error)),
        // Options specific to prefix commands, i.e. commands invoked via chat messages
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some(String::from(config.prefix())),

            mention_as_prefix: true,
            // An edit tracker needs to be supplied here to make edit tracking in commands work
            edit_tracker: Some(poise::EditTracker::for_timespan(
                std::time::Duration::from_secs(3600 * 3),
            )),
            ..Default::default()
        },
        ..Default::default()
    }
}
