use lazy_static::lazy_static;
use regex::Regex;
use serenity::framework::standard::{macros::command, CommandResult};
use serenity::prelude::*;
use serenity::{framework::standard::Args, model::prelude::*};
use tokio::task;

lazy_static! {
    static ref URL_REGEX: Regex = Regex::new(r"(http://www\.|https://www\.|http://|https://)?[a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,5}(:[0-9]{1,5})?(/.*)?").expect("Couldn't build URL Regex");
    static ref ARG_REGEX: Regex = Regex::new(r"(--?[a-zA-Z\-]+)(\([a-zA-Z0-9\-]+\))?").expect("Couldn't build args Regex");
}

#[command("youtube-dl")]
async fn ytd(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if args.len() != 1 {
        msg.reply(&ctx.http, "Please provide a link to a video source")
            .await?;
        return Ok(());
    }

    let url = args.single::<String>()?;

    if !URL_REGEX.is_match(&url) {
        msg.reply(&ctx.http, format!("{} is not a valid url", url))
            .await?;
        return Ok(());
    }

    let id = msg.author.id.as_u64();

    task::spawn(crate::model::youtubedl::start_download(
        msg.clone(),
        id.clone(),
        ctx.http.clone(),
        url,
    ));
    Ok(())
}
