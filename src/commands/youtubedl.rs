use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use ytd_rs::ytd::YoutubeDL;

#[command]
async fn ytd(ctx: &Context, msg: &Message) -> CommandResult {
    let content = msg.content_safe(&ctx.cache).await;
    let args = content.split(" ").collect::<Vec<&str>>();
    if let Ok(ytd) = YoutubeDL::new(
        "tmp/",
        vec![],
        args.get(args.len() - 1).unwrap().to_string(),
    ) {
        let dir = match ytd.download() {
            Ok(dir) => dir,
            Err(why) => {
                msg.reply(&ctx.http, format!("{}", why)).await?;
                return Ok(());
            }
        };
    }

    Ok(())
}
