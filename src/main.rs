// models like music manager, yt downloader...
pub mod model {
    pub mod music;
}

// commands
mod commands;

use serenity::{
    async_trait,
    client::bridge::gateway::ShardManager,
    framework::{standard::macros::group, StandardFramework},
    http::Http,
    model::{event::ResumedEvent, gateway::Ready},
    prelude::*,
};
use std::{collections::HashSet, env, sync::Arc};

use tracing::{error, info};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use commands::{general::*, music::*};

pub struct ShardManagerContainer;

// shard manager
impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct Handler;

// Ready and Resumed events to notify if the bot has started/resumed
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

#[group]
#[commands(ping)]
struct General;

#[group]
#[commands(play, test)]
struct Music;

#[tokio::main]
async fn main() {
    // load environment
    dotenv::dotenv().expect("Failed to load environment");

    // init the logger to use environment variables
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to start the Logger");

    let token =
        env::var("DISCORD_TOKEN").expect("Failed to load DISCORD_TOKEN from the environment");

    let http = Http::new_with_token(&token);

    // get owners and bot id from application
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Create bot
    // TODO: add commands
    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix("?"))
        .group(&GENERAL_GROUP)
        .group(&MUSIC_GROUP);

    let mut client = Client::builder(&token)
        .framework(framework)
        .event_handler(Handler)
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
