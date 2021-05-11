use std::{
    fs::{create_dir_all, read_dir, remove_dir_all},
    path::PathBuf,
};

use fs_extra::dir::{self, get_size};
use lazy_static::lazy_static;
use regex::Regex;
use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use sha2::{Digest, Sha256};
use ytd_rs::ytd::{Arg, YoutubeDL};

lazy_static! {
    static ref URL_REGEX: Regex = Regex::new(r"(http://www\.|https://www\.|http://|https://)?[a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,5}(:[0-9]{1,5})?(/.*)?").expect("Couldn't build URL Regex");
    static ref ARG_REGEX: Regex = Regex::new(r"(--?[a-zA-Z\-]+)(\([a-zA-Z0-9\-]+\))?").expect("Couldn't build args Regex");
}

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

    let mut dir = crate::BOT_DIR.clone();
    dir.push("tmp");
    dir.push("ytd");
    dir.push(id.to_string());

    let download = match download(&dir, args, link).await {
        Ok(dl) => dl,
        Err(why) => {
            msg.reply(&ctx.http, format!("Error: {}", why)).await?;
            let _ = remove_dir_all(dir.as_path());
            return Ok(());
        }
    };

    let size = match get_size(dir.as_path()) {
        Ok(size) => size,
        Err(why) => {
            msg.reply(&ctx.http, format!("Error: {}", why)).await?;
            return Ok(());
        }
    };

    if size < 8000000 {
        let _ = send_files_to_channel(msg, ctx, download).await;
    } else {
        let _ = send_files_to_webserver(msg, ctx, download, &id).await;
    }
    let _ = remove_dir_all(dir.as_path());
    Ok(())
}

async fn download(
    bot_dir: &PathBuf,
    args: Vec<Arg>,
    link: String,
) -> std::result::Result<Vec<PathBuf>, String> {
    //TODO: hash user id
    // let mut hasher = Sha256::new();
    // hasher.update(id.to_string());
    // let hash = hasher.finalize();
    // match std::str::from_utf8(&hash[..]) {
    //     Ok(hash) => bot_dir.push(hash),
    //     Err(_) => {
    //         return Err("couldn't get user id".to_string());
    //     }
    // };

    if bot_dir.exists() {
        match read_dir(&bot_dir) {
            Ok(read) => {
                if read.count() != 0 {
                    return Err("download running".to_string());
                }
            }
            Err(why) => {
                return Err(format!("could't read download directory: {:?}", why));
            }
        }
    }

    let ytd = match bot_dir.to_str() {
        Some(path) => match YoutubeDL::new(path, args, link) {
            Ok(ytd) => ytd,
            Err(why) => {
                return Err(format!("couldn't create download: {:?}", why));
            }
        },
        None => return Err("couldn't get directory for download".to_string()),
    };

    let download = match ytd.download() {
        Ok(dl) => dl,
        Err(why) => {
            return Err(format!("couldn't start download: {:?}", why));
        }
    };

    match get_all_files(download) {
        Ok(files) => Ok(files),
        Err(_) => {
            return Err("couldn't read download dir".to_string());
        }
    }
}

async fn send_files_to_channel(msg: &Message, ctx: &Context, files: Vec<PathBuf>) -> CommandResult {
    match msg.channel(&ctx.cache).await {
        Some(ch) => {
            if files.is_empty() {
                msg.reply(&ctx.http, "Error: Couldn't download files")
                    .await?;
            }
            ch.id()
                .send_files(&ctx.http, &files, |m| m.content("Here are your files:"))
                .await?;
        }
        None => {
            msg.reply(&ctx.http, "Error: couldn't send files to channel")
                .await?;
        }
    };
    Ok(())
}

async fn send_files_to_webserver(
    msg: &Message,
    ctx: &Context,
    files: Vec<PathBuf>,
    id: &u64,
) -> CommandResult {
    let host = match crate::CONFIG.get_str("hostname") {
        Ok(host) => host,
        Err(_) => {
            msg.reply(&ctx.http, "Error: couldn't find hostname in config.yml")
                .await?;
            return Ok(());
        }
    };
    let webdir = match crate::CONFIG.get_str("webdir") {
        Ok(host) => host,
        Err(_) => {
            msg.reply(&ctx.http, "Error: couldn't find hostname in config.yml")
                .await?;
            return Ok(());
        }
    };
    let webroot = match crate::CONFIG.get_str("webroot") {
        Ok(root) => root,
        Err(_) => {
            msg.reply(&ctx.http, "Error, couldn't find webroot in config.yml")
                .await?;
            return Ok(());
        }
    };

    let final_dir = PathBuf::from(format!("{}/{}/{}", webroot, webdir, id));
    if !final_dir.exists() {
        if let Err(_) = create_dir_all(&final_dir) {
            msg.reply(
                &ctx.http,
                "Error: couldn't create user's download directory, contact bot owner",
            )
            .await?;
            return Ok(());
        }
    }
    if let Err(_) = fs_extra::move_items(&files, &final_dir, &dir::CopyOptions::new()) {
        msg.reply(
            &ctx.http,
            "Error: couldn't move files to destination, contact bot owner",
        )
        .await?;
        return Ok(());
    }
    msg.reply(
        &ctx.http,
        format!("You can download your files here {}/{}", host, webdir),
    )
    .await?;
    Ok(())
}

//TODO: reimplement that beauty
fn get_all_files(file: &PathBuf) -> Result<Vec<PathBuf>, String> {
    let mut files: Vec<PathBuf> = Vec::new();
    if file.is_dir() {
        for path in match read_dir(file.as_path()) {
            Ok(read) => read,
            Err(_) => {
                return Err("couldn't read download directory".to_string());
            }
        } {
            match path {
                Ok(p) => match get_all_files(&p.path().to_path_buf()) {
                    Ok(vec) => {
                        for f in vec {
                            files.push(f);
                        }
                    }
                    Err(why) => {
                        return Err(why);
                    }
                },
                Err(_) => {
                    return Err("couldn't read download directory".to_string());
                }
            };
        }
    } else {
        files.push(file.clone());
    }
    Ok(files)
}

fn get_args(message: String) -> std::result::Result<(Vec<Arg>, String), String> {
    let mut args: Vec<Arg> = Vec::new();
    let mut link = "".to_string();

    let user_inp: &str = match message.split("ytd").collect::<Vec<&str>>().last() {
        Some(inp) => inp,
        None => return Err("couldn't get user input".to_string()),
    };

    for s in user_inp.trim().split(" ").collect::<Vec<&str>>().iter() {
        if URL_REGEX.is_match(s) {
            if !link.eq("") {
                return Err("you can only download one source at a time!".to_string());
            }
            link = s.to_string();
            continue;
        }
        match ARG_REGEX.captures(s) {
            Some(cap) => {
                let arg = cap.get(1).map_or("", |m| m.as_str());
                let inp = cap.get(2).map_or("", |m| m.as_str());
                if !inp.eq("") {
                    args.push(Arg::new_with_arg(
                        &arg,
                        &inp.replace("(", "").replace(")", ""),
                    ));
                } else {
                    args.push(Arg::new(&arg));
                }
            }
            None => {
                return Err("{} is not an option".to_string());
            }
        }
    }

    Ok((args, link))
}
