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

use commands::*;
use error::{Error as AYError, Sendable};

const TOML_CONFIG: &'static str = "config.toml";
const JSON_CONFIG: &'static str = "config.json";
const ENV_PREFIX: &'static str = "DISCORD_";
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
pub struct Data;

/// Show this help menu
#[poise::command(prefix_command, track_edits, slash_command)]
async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            show_context_menu_commands: true,
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}

#[poise::command(prefix_command, track_edits, slash_command)]
async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("pong!").await?;
    Ok(())
}

#[poise::command(prefix_command, track_edits, slash_command)]
async fn pingerror(_ctx: Context<'_>) -> Result<(), Error> {
    Err(Box::new(error::Error::InvalidInput("test exception")))
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx } => {
            if let Some(ayerr) = error.downcast_ref::<AYError>() {
                // let _ = ctx.say(format!("{}", ayerr)).await;
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
        ..Default::default()
    };

    Ok(poise::Framework::builder()
        .token(config.discord_token.to_string())
        .user_data_setup(move |_ctx, _ready, framework| {
            register_signal_handler(framework.shard_manager().clone());
            Box::pin(async move { Ok(Data) })
        })
        .options(options)
        .intents(
            serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT,
        )
        .run()
        .await?)
}

fn register_signal_handler(shard_manager: Arc<serenity::Mutex<serenity::ShardManager>>) {
    let sm = shard_manager.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        tracing::info!("Recieved ctrl+c signal, shutting down...");
        sm.lock().await.shutdown_all().await;
    });
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
