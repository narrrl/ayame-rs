use crate::ShardManagerContainer;
use lazy_static::lazy_static;
use regex::Regex;
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::Color;
use serenity::{
    client::bridge::gateway::ShardId,
    framework::standard::{macros::command, Args, CommandResult},
};
use tokio::task;

lazy_static! {
    pub static ref URL_REGEX: Regex = Regex::new(r"(http://www\.|https://www\.|http://|https://)?[a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,5}(:[0-9]{1,5})?(/.*)?").expect("Couldn't build URL Regex");
}

#[command("youtube-dl")]
#[bucket = "really_slow"]
#[aliases("ytd", "dl")]
async fn ytd(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if args.len() < 1 || args.len() > 2 {
        msg.reply(&ctx.http, "Please provide a single link to a video source")
            .await?;
        return Ok(());
    }
    let url = args.single::<String>()?;
    let mut as_audio = false;
    if args.len() == 2 {
        let option = args.single::<String>()?;

        if option.eq("-audio") {
            as_audio = true;
        }
    }

    if !URL_REGEX.is_match(&url) {
        msg.reply(&ctx.http, format!("{} is not a valid url", url))
            .await?;
        return Ok(());
    }

    let id = msg.author.id.as_u64();

    task::spawn(crate::model::youtubedl::start_download(
        msg.channel_id.clone(),
        id.clone(),
        ctx.http.clone(),
        url,
        as_audio,
    ));
    Ok(())
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let shard_manager = match data.get::<ShardManagerContainer>() {
        Some(shard_manager) => shard_manager,
        None => {
            msg.reply(ctx, "There was a problem getting the shard manager")
                .await?;

            return Ok(());
        }
    };
    let manager = shard_manager.lock().await;
    let runners = manager.runners.lock().await;

    let runner = match runners.get(&ShardId(ctx.shard_id)) {
        Some(runner) => runner,
        None => {
            msg.reply(ctx, "No shard found").await?;

            return Ok(());
        }
    };

    match runner.latency {
        Some(latency) => {
            msg.reply(&ctx.http, &format!("Pong! {:?}", latency))
                .await?
        }
        None => msg.reply(&ctx.http, "Pong!").await?,
    };
    Ok(())
}

#[command]
async fn invite(ctx: &Context, msg: &Message) -> CommandResult {
    let application_id = crate::CONFIG
        .get_str("application_id")
        .expect("expected application_id in config.yml");
    let invite_link = format!("https://discord.com/api/oauth2/authorize?client_id={}&permissions=8&scope=applications.commands%20bot", application_id);
    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title("Invite the bot to your server");
                e.url(invite_link);
                e.image("https://media1.tenor.com/images/6f0ba23f8a1abe87629c1309bdaa57d7/tenor.gif?itemid=20472559");
                e.color(Color::from_rgb(238, 14, 97));
                e
            })
        })
        .await?;

    Ok(())
}
