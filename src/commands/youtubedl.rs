use std::{
    io::Result,
    path::{Path, PathBuf},
};

use std::fs;

use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use ytd_rs::ytd::YoutubeDL;

#[command]
async fn ytd(ctx: &Context, msg: &Message) -> CommandResult {
    let content = msg.content_safe(&ctx.cache).await;
    let id = msg.author.id.as_u64();

    let (args, link) = match get_args(content) {
        Ok(tup) => tup,
        Err(why) => {
            msg.reply(&ctx.http, format!("Error: {}", why)).await?;
            return Ok(());
        }
    };

    let dl = match download(&ctx, id, args, link).await {
        Ok(dl) => dl,
        Err(why) => {
            msg.reply(&ctx.http, format!("{}", why)).await?;

            return Ok(());
        }
    };

    if let Some(files) = dl {
        match msg.channel(&ctx.cache).await {
            Some(ch) => {
                ch.id()
                    .send_files(&ctx.http, &files, |m| m.content("Here are your files:"))
                    .await?;
            }
            None => {
                msg.reply(&ctx.http, "Couldn't send files to channel")
                    .await?;
            }
        };
        return Ok(());
    } else {
        let host = match crate::CONFIG.get_str("hostname") {
            Ok(host) => host,
            Err(_) => {
                msg.reply(&ctx.http, "Couldn't find hostname in config.yml")
                    .await?;
                return Ok(());
            }
        };
        let webdir = match crate::CONFIG.get_str("webdir") {
            Ok(host) => host,
            Err(_) => {
                msg.reply(&ctx.http, "Couldn't find hostname in config.yml")
                    .await?;
                return Ok(());
            }
        };
        msg.reply(
            &ctx.http,
            format!("You can download your files here {}/{}", host, webdir),
        )
        .await?;
    }
    Ok(())
}

async fn download(
    ctx: &Context,
    id: &u64,
    args: Vec<String>,
    link: String,
) -> Result<Option<Vec<PathBuf>>> {
    Ok(None)
}

fn get_args(message: String) -> Result<(Vec<String>, String)> {
    let mut args = Vec::new();
    let mut link = String::new();

    for s in message.split(" ").collect::<Vec<&str>>().iter() {}

    Ok((args, link))
}

fn size_of_dir(file: &Path) -> Result<u64> {
    let mut size: u64 = 0;

    if file.is_dir() {
        for entry in fs::read_dir(file)? {
            let entry = entry?;
            let path = entry.path();
            size += match size_of_dir(&path) {
                Ok(s) => s,
                Err(err) => {
                    return Err(err);
                }
            };
        }
    } else {
        size = match fs::metadata(file) {
            Ok(data) => data.len(),
            Err(err) => {
                return Err(err);
            }
        };
    }
    Ok(size)
}
