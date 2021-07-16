use std::fs::remove_dir_all;
use std::{fs::read_dir, path::PathBuf, sync::Arc};

use fs_extra::dir::get_size;
use lazy_static::lazy_static;
use regex::Regex;
use serenity::model::prelude::*;
use serenity::utils::Color;
use serenity::{framework::standard::CommandResult, http::Http};
use ytd_rs::{ResultType, YoutubeDL};

lazy_static! {
    pub static ref URL_REGEX: Regex = Regex::new(r"(http://www\.|https://www\.|http://|https://)?[a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,5}(:[0-9]{1,5})?(/.*)?").expect("Couldn't build URL Regex");
    pub static ref MAX_DISCORD_FILE_SIZE: u64 = 8_000_000; // 8mb
    pub static ref MAX_FILE_SIZE: u64 = 200_000_000; // 200mb
}

pub async fn start_download(msg: Message, id: u64, http: Arc<Http>, url: String) -> CommandResult {
    let dir = create_download_dir(id).await?;
    if let Ok(read) = read_dir(&dir) {
        if read.count() != 0 {
            // we don't want the directory to be cleaned
            // because a download is running
            return send_error(&msg, &http, "download running").await;
        }
    }

    let mut update_message = msg
        .channel_id
        .send_message(&http, |m| m.content("Starting download ..."))
        .await?;
    let file = match make_download(&dir, &url).await {
        Err(why) => {
            remove_dir_all(&dir)?;
            return send_error(&msg, &http, &why).await;
        }
        Ok(path) => path,
    };

    let size = match get_size(file.as_path()) {
        Ok(size) => size,
        Err(why) => {
            remove_dir_all(&dir)?;
            return send_error(&msg, &http, &format!("{}", why)).await;
        }
    };

    if size < *MAX_FILE_SIZE && size < *MAX_DISCORD_FILE_SIZE {
        update_message
            .edit(&http, |m| m.content("Uploading to Discord ..."))
            .await?;
        send_file_to_channel(file, &msg, &http).await?;
    } else if size < *MAX_FILE_SIZE {
        update_message
            .edit(&http, |m| m.content("Uploading to transfer.sh ..."))
            .await?;
        send_file_to_transfersh(file, &msg, &http, &id.to_string()).await?;
    } else {
        send_error(&msg, &http, "Your download was larger than 100mb").await?;
    }
    remove_dir_all(&dir)?;

    Ok(())
}

async fn make_download(dir: &PathBuf, url: &str) -> Result<PathBuf, String> {
    // get the youtubedl task
    let ytd: YoutubeDL = match dir.to_str() {
        Some(path) => YoutubeDL::new(path, vec![], url)?,
        None => return Err("couldn't get directory for download".to_string()),
    };

    let file = get_downloaded_file(ytd, &url).await?;

    Ok(file)
}

async fn create_download_dir(id: u64) -> Result<PathBuf, String> {
    // tmp download directory is
    // {bot_dir}/tmp/ytd/id
    let mut dir = crate::BOT_DIR.clone();
    dir.push("tmp");
    dir.push("ytd");
    dir.push(format!("{}", id));
    Ok(dir)
}

async fn get_downloaded_file(ytd: YoutubeDL, url: &str) -> Result<PathBuf, String> {
    let result = ytd.download();

    let path = match result.result_type() {
        ResultType::SUCCESS => result.output_dir(),
        ResultType::IOERROR | ResultType::FAILURE => {
            return Err(format!("Couldn't download {}", url));
        }
    };

    let dir_entry = match read_dir(path.as_path()) {
        Ok(read) => read,
        Err(_) => {
            return Err("couldn't read download directory".to_string());
        }
    };

    for entry in dir_entry {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_file() {
                return Ok(path);
            }
        }
    }
    Err("Couldn't find downloaded file".to_string())
}

async fn send_file_to_channel(file: PathBuf, msg: &Message, http: &Arc<Http>) -> CommandResult {
    msg.channel_id
        .send_files(&http, &vec![file], |m| m.content(""))
        .await?;
    Ok(())
}

async fn send_file_to_transfersh(
    file: PathBuf,
    msg: &Message,
    http: &Arc<Http>,
    safe_name: &str,
) -> CommandResult {
    let link = crate::model::upload_file(&file, safe_name)?;
    msg.channel_id
        .send_message(&http, |m| m.content(link))
        .await?;
    Ok(())
}

async fn send_error(msg: &Message, http: &Arc<Http>, error_msg: &str) -> CommandResult {
    msg.channel_id
        .send_message(&http, |m| {
            m.embed(|e| {
                e.title("Error");
                e.description(error_msg);
                e.color(Color::from_rgb(238, 14, 97));
                e
            })
        })
        .await?;
    Ok(())
}
