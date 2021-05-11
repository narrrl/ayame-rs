use std::{
    fs::{create_dir_all, read_dir, remove_dir_all},
    path::PathBuf,
    sync::Arc,
};

use fs_extra::dir::{self, get_size};
use lazy_static::lazy_static;
use regex::Regex;
use serenity::prelude::*;
use serenity::{client::Cache, model::prelude::*};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    http::Http,
};
use sha2::{Digest, Sha256};
use tokio::task;
use ytd_rs::ytd::{Arg, YoutubeDL};

lazy_static! {
    static ref URL_REGEX: Regex = Regex::new(r"(http://www\.|https://www\.|http://|https://)?[a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,5}(:[0-9]{1,5})?(/.*)?").expect("Couldn't build URL Regex");
    static ref ARG_REGEX: Regex = Regex::new(r"(--?[a-zA-Z\-]+)(\([a-zA-Z0-9\-]+\))?").expect("Couldn't build args Regex");
}

#[command]
async fn ytd(ctx: &Context, msg: &Message) -> CommandResult {
    let content = msg.content_safe(&ctx.cache).await;
    let id = msg.author.id.as_u64();
    // create hash from id
    let mut hasher = Sha256::new();
    hasher.update(id.to_string());
    let hash = hasher.finalize();
    let hashed_id = &format!("{:x}", &hash);

    // get arguments and the download link
    let (args, link) = match get_args(content) {
        Ok(tup) => tup,
        Err(why) => {
            msg.reply(&ctx.http, format!("Error: {}", why)).await?;
            return Ok(());
        }
    };

    // tmp download directory is
    // {bot_dir}/tmp/ytd/{hashed_id}
    let mut dir = crate::BOT_DIR.clone();
    dir.push("tmp");
    dir.push("ytd");
    dir.push(hashed_id);

    // clone values for passing into another thread
    let cache = ctx.cache.clone();
    let http = ctx.http.clone();

    // spawn a new thread that handles the download
    // because we don't want to block this thread
    task::spawn(start(
        http,
        cache,
        msg.clone(),
        dir,
        args,
        link,
        hashed_id.to_string(),
    ));
    Ok(())
}

async fn start(
    http: Arc<Http>,
    cache: Arc<Cache>,
    msg: Message,
    dir: PathBuf,
    args: Vec<Arg>,
    link: String,
    hashed_id: String,
) -> CommandResult {
    let download = match download(&dir, args, link).await {
        Ok(dl) => dl,
        Err((clean_dir, why)) => {
            msg.reply(&http, format!("Error: {}", &why)).await?;

            if clean_dir {
                let _ = remove_dir_all(dir.as_path());
            }
            return Ok(());
        }
    };

    let size = match get_size(dir.as_path()) {
        Ok(size) => size,
        Err(why) => {
            msg.reply(&http, format!("Error: {}", why)).await?;
            return Ok(());
        }
    };

    if size < 8000000 {
        send_files_to_channel(&msg, http, cache, download).await?;
    } else {
        send_files_to_webserver(&msg, http, download, &hashed_id).await?;
    }
    let _ = remove_dir_all(dir.as_path());
    Ok(())
}

///
/// Creates a youtube download via the ytd-rs crate
///
/// Simply downloads all files and returns a list with ['PathBuf'] to all files
///
///
/// Why the fuck is the Error a tuple of (bool, String), you ask?
/// Great qustion, because the download tmp directory gets cleaned on error,
/// but in this case we sometime don't want this directory to be cleaned.
/// For example if the error is that a download is already running.
/// A clean would mean that we delete the files of the other download :(
async fn download(
    bot_dir: &PathBuf,
    args: Vec<Arg>,
    link: String,
) -> std::result::Result<Vec<PathBuf>, (bool, String)> {
    // check if another download by that user is running
    if bot_dir.exists() {
        match read_dir(&bot_dir) {
            Ok(read) => {
                if read.count() != 0 {
                    // we don't want the directory to be cleaned
                    return Err((false, "download running".to_string()));
                }
            }
            Err(why) => {
                return Err((true, format!("could't read download directory: {:?}", why)));
            }
        }
    }

    // get the youtubedl task
    let ytd = match bot_dir.to_str() {
        Some(path) => match YoutubeDL::new(path, args, link) {
            Ok(ytd) => ytd,
            Err(why) => {
                return Err((true, format!("couldn't create download: {:?}", why)));
            }
        },
        None => return Err((true, "couldn't get directory for download".to_string())),
    };

    // download via youtubedl and get the directory
    let download = match ytd.download() {
        Ok(dl) => dl,
        Err(why) => {
            return Err((true, format!("couldn't start download: {:?}", why)));
        }
    };

    // get all files in that directory that aren't directories and return them as result
    match get_all_files(download) {
        Ok(files) => Ok(files),
        Err(_) => {
            return Err((true, "couldn't read download dir".to_string()));
        }
    }
}

async fn send_files_to_channel(
    msg: &Message,
    http: Arc<Http>,
    cache: Arc<Cache>,
    files: Vec<PathBuf>,
) -> CommandResult {
    // simply send alle files to the channel
    // becareful when the files are succeeding a size of around 8mb
    match msg.channel(&cache).await {
        Some(ch) => {
            if files.is_empty() {
                msg.reply(&http, "Error: Couldn't download files").await?;
            }
            ch.id()
                .send_files(&http, &files, |m| m.content("Here are your files:"))
                .await?;
        }
        None => {
            msg.reply(&http, "Error: couldn't send files to channel")
                .await?;
        }
    };
    Ok(())
}

// TODO: make configuration to disable that feature
async fn send_files_to_webserver(
    msg: &Message,
    http: Arc<Http>,
    files: Vec<PathBuf>,
    id: &str,
) -> CommandResult {
    // check if this option was disabled in config
    if crate::CONFIG
        .get_bool("disableWebserver")
        .map_or(false, |m| m)
    {
        msg.reply(
            &http,
            "Bot owner disabled the option to upload files larger than 8mb",
        )
        .await?;
        return Ok(());
    }
    // first get all the configuration from the config.yml
    let host = match crate::CONFIG.get_str("hostname") {
        Ok(host) => host,
        Err(_) => {
            msg.reply(&http, "Error: couldn't find hostname in config.yml")
                .await?;
            return Ok(());
        }
    };
    let webdir = match crate::CONFIG.get_str("webdir") {
        Ok(host) => host,
        Err(_) => {
            msg.reply(&http, "Error: couldn't find hostname in config.yml")
                .await?;
            return Ok(());
        }
    };
    let webroot = match crate::CONFIG.get_str("webroot") {
        Ok(root) => root,
        Err(_) => {
            msg.reply(&http, "Error, couldn't find webroot in config.yml")
                .await?;
            return Ok(());
        }
    };
    // this is the path to the download directory inside the webroot
    // {webroot}/{webdir}/{id}
    // for example /var/www/html/discord/tmp/{user_id}
    let final_dir = PathBuf::from(format!("{}/{}/{}", webroot, webdir, id));
    // create if it doesn't exists
    if !final_dir.exists() {
        if let Err(_) = create_dir_all(&final_dir) {
            msg.reply(
                &http,
                "Error: couldn't create user's download directory, contact bot owner",
            )
            .await?;
            return Ok(());
        }
    }

    // move all files to the webroot
    if let Err(_) = fs_extra::move_items(&files, &final_dir, &dir::CopyOptions::new()) {
        msg.reply(
            &http,
            "Error: couldn't move files to destination, contact bot owner",
        )
        .await?;
        return Ok(());
    }
    // inform user about the download
    // {hostname}/{webdir}/{id}/
    msg.reply(
        &http,
        format!(
            "You can download your files here {}/{}/{}/",
            host, webdir, id
        ),
    )
    .await?;
    Ok(())
}

/// get all non directory files recursively
fn get_all_files(file: &PathBuf) -> Result<Vec<PathBuf>, String> {
    let mut files: Vec<PathBuf> = Vec::new();
    // check if file is dir
    if file.is_dir() {
        // get dir entries
        let dir_entry = match read_dir(file.as_path()) {
            Ok(read) => read,
            Err(_) => {
                return Err("couldn't read download directory".to_string());
            }
        };

        // iterate over dir entries
        for path in dir_entry {
            let p = match path {
                Ok(p) => p,
                Err(_) => {
                    return Err("couldn't read download directory".to_string());
                }
            };

            // get all files inside that file
            // or just that file if its not a directory
            match get_all_files(&p.path().to_path_buf()) {
                Ok(vec) => {
                    for f in vec {
                        // push it into the final file list
                        files.push(f);
                    }
                }
                Err(why) => {
                    return Err(why);
                }
            };
        }
    } else {
        // if file isn't a dir, push it and return
        files.push(file.clone());
    }
    // return all files as vector
    Ok(files)
}

fn get_args(message: String) -> std::result::Result<(Vec<Arg>, String), String> {
    let mut args: Vec<Arg> = Vec::new();
    // download rate limit
    args.push(Arg::new_with_arg(
        "-r",
        crate::CONFIG
            .get_str("downloadRateLimit")
            .map_or("1000K".to_string(), |m| m)
            .as_ref(),
    ));
    let mut link = "".to_string();

    // split into 2 at the first "ytd" inside the userinput to separate
    // {prefix}ytd from the {args.../link}
    let user_inp: &str = match message.splitn(2, "ytd").collect::<Vec<&str>>().last() {
        Some(inp) => inp,
        None => return Err("couldn't get user input".to_string()),
    };

    // trim the user input and split at all whitespaces
    for s in user_inp.trim().split_whitespace() {
        // check if an link was already found
        // because we don't want mass downloads
        if URL_REGEX.is_match(s) {
            if !link.eq("") {
                return Err("you can only download one source at a time!".to_string());
            }
            link = s.to_string();
            continue;
        }
        // check if its an arg with input or not
        // for example --extract-audio is an arg
        // but --audio-format({audio-format}) needs an audio format as input
        match ARG_REGEX.captures(s) {
            Some(cap) => {
                // inp is "" when its just an arg
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
            // if nothing matches inform user that this is not a valid option
            None => {
                return Err(format!("{} is not an option", s));
            }
        }
    }

    Ok((args, link))
}
