use figment::{
    providers::{Env, Format, Json, Toml},
    Figment,
};
use poise::{futures_util::future::join_all, serenity_prelude as serenity};
use serde::Deserialize;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub mod error;

const TOML_CONFIG: &'static str = "config.toml";
const JSON_CONFIG: &'static str = "config.json";
const ENV_PREFIX: &'static str = "AYAME_";

// configuration
#[derive(Deserialize, Debug, PartialEq)]
pub struct Config {
    discord_token: String,
    youtube_token: Option<String>,
    swfr_token: Option<String>,
    prefix: Option<String>,
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
            if let Some(ayerr) = error.downcast_ref::<error::Error>() {
                let _ = ctx.say(format!("{}", ayerr)).await;
            } else {
                println!("Error in command `{}`: {:?}", ctx.command().name, error,);
            }
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config: Config = Figment::new()
        .merge(Env::prefixed(ENV_PREFIX))
        .merge(Toml::file(TOML_CONFIG))
        .merge(Json::file(JSON_CONFIG))
        .extract()?;

    let mut handles = vec![];
    handles.push(tokio::spawn(async move {
        if let Err(why) = run_discord(&config).await {
            tracing::error!("Error: failed to start discord {:#?}", why);
        }
    }));
    join_all(handles).await;
    Ok(())
}

async fn run_discord(config: &Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = poise::FrameworkOptions {
        commands: vec![help(), ping(), pingerror()],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some(String::from(
                &config.prefix.clone().unwrap_or(String::from("~")),
            )),
            edit_tracker: Some(poise::EditTracker::for_timespan(
                std::time::Duration::from_secs(3600),
            )),
            additional_prefixes: vec![],
            ..Default::default()
        },
        /// The global error handler for all error cases that may occur
        on_error: |error| Box::pin(on_error(error)),
        /// This code is run before every command
        pre_command: |ctx| {
            Box::pin(async move {
                println!("Executing command {}...", ctx.command().qualified_name);
            })
        },
        /// This code is run after a command if it was successful (returned Ok)
        post_command: |ctx| {
            Box::pin(async move {
                println!("Executed command {}!", ctx.command().qualified_name);
            })
        },
        /// Every command invocation must pass this check to continue execution
        command_check: Some(|ctx| {
            Box::pin(async move {
                if ctx.author().id == 123456789 {
                    return Ok(false);
                }
                Ok(true)
            })
        }),
        listener: |_ctx, event, _framework, _data| {
            Box::pin(async move {
                println!("Got an event in listener: {:?}", event.name());
                Ok(())
            })
        },
        ..Default::default()
    };

    Ok(poise::Framework::build()
        .token(config.discord_token.to_string())
        .user_data_setup(move |_ctx, _ready, framework| {
            register_signal_handler(framework.shard_manager());
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
        tracing::info!("Recieved ctrl+c signal");
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
        shard_manager.lock().await.shutdown_all().await;
    });
}
