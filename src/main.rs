use figment::{
    providers::{Env, Format, Json, Toml},
    Figment,
};
use lazy_static::lazy_static;
use poise::serenity_prelude as serenity;
use serde::Deserialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub mod commands;
pub mod error;
pub mod menu;
pub mod util;

// `commands::mod.rs` re-exports all commands for easy importing
use commands::*;
use error::{Error as AYError, Sendable};

const TOML_CONFIG: &'static str = "config.toml";
const JSON_CONFIG: &'static str = "config.json";
const ENV_PREFIX: &'static str = "AYAME_";
const DEFAULT_COLOR: &'static str = "23272A";
const BASE_16: u32 = 16;

// configuration
#[derive(Deserialize, Debug, PartialEq)]
pub struct Config {
    discord_token: String,
    youtube_token: Option<String>,
    swfr_token: Option<String>,
    prefix: Option<String>,
    color: Option<String>,
}

// some global stuff like configuration etc.
lazy_static! {
    // we use a static reference to our config
    pub static ref CONFIG: Config = {
        Figment::new()
            .merge(Env::prefixed(ENV_PREFIX))
            .merge(Toml::file(TOML_CONFIG))
            .merge(Json::file(JSON_CONFIG))
            .extract()
            .expect("Couldn't create config")
    };
    // static color for easy access
    pub static ref COLOR: serenity::Colour = {
        let color = u32::from_str_radix(
            &CONFIG
                .color
                .clone()
                .unwrap_or(String::from(DEFAULT_COLOR))
                // if config is like #000000
                .replace("#", "")
                // if config is like 0x000000
                .replace("0x", ""),
            BASE_16,
        )
        .expect("Couldn t convert color in config");
        serenity::Colour::new(color)
    };
}

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// Custom user data passed to all command functions
// might be expanded in the future
#[non_exhaustive]
pub struct Data;

/// custom event listener
async fn event_listener(
    _ctx: &serenity::Context,
    event: &poise::Event<'_>,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    _data: &Data,
) -> Result<(), Error> {
    match event {
        poise::Event::Ready { data_about_bot } => {
            tracing::info!("{} is connected!", data_about_bot.user.name);
            tracing::info!("Total Guilds: {}", data_about_bot.guilds.len())
        }
        _ => {}
    }

    Ok(())
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx } => {
            if let Some(ayerr) = error.downcast_ref::<AYError>() {
                // notify user
                if let Err(e) = ayerr.send(&ctx).await {
                    tracing::error!("Error while handling error: {}", e);
                }
            } else {
                tracing::error!("Error in command `{}`: {:?}", ctx.command().name, error,);
            }
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                tracing::error!("Error while handling error: {}", e);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // init tracing
    tracing_subscriber::fmt()
        .pretty()
        // .with_max_level(tracing::Level::INFO)
        .with_thread_names(true)
        .init();
    // run the discord client with the configuration
    // we don't actually need to pass the config because its global
    // but that way we can ensure that this is the first time the config is used
    // because lazy static is kinda like a singleton (setups config which can fail and stores it in
    // heap for easy access)
    run_discord(&CONFIG).await
}

async fn run_discord(config: &Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = poise::FrameworkOptions {
        commands: vec![
            help(),
            ping(),
            pingerror(),
            avatar(),
            uwu(),
            uwuify(),
            register(),
            invite(),
            shutdown(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some(String::from(
                &config.prefix.clone().unwrap_or(String::from("~")),
            )),
            edit_tracker: Some(poise::EditTracker::for_timespan(
                std::time::Duration::from_secs(3600),
            )),
            ..Default::default()
        },
        /// The global error handler for all error cases that may occur
        on_error: |error| Box::pin(on_error(error)),
        listener: |ctx, event, framework, user_data| {
            Box::pin(event_listener(ctx, event, framework, user_data))
        },
        ..Default::default()
    };

    Ok(poise::Framework::builder()
        .token(config.discord_token.to_string())
        .user_data_setup(move |_ctx, _ready, framework| {
            // we register signal handlers for sigterm, ctrl+c, ...
            // TODO: better way to do this?
            register_signal_handler(framework.shard_manager().clone());
            // create user data
            Box::pin(async move { Ok(Data) })
        })
        .options(options)
        .intents(
            serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT,
        )
        .run()
        .await?)
}

/// this funcitons registers all the signal handlers
/// for example sigterm to shutdown the bot the right way
fn register_signal_handler(shard_manager: Arc<serenity::Mutex<serenity::ShardManager>>) {
    let sm = shard_manager.clone();

    // ctrl+c
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        tracing::info!("Recieved ctrl+c signal, shutting down...");
        sm.lock().await.shutdown_all().await;
    });

    // sigterm
    tokio::spawn(async move {
        let term = Arc::new(AtomicBool::new(false));

        if let Err(why) =
            signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))
        {
            tracing::error!("couldn't register sigterm hook {:?}", why);
            return;
        }
        while !term.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
        tracing::info!("Recieved sigterm, shutting down...");
        shard_manager.lock().await.shutdown_all().await;
    });
}
