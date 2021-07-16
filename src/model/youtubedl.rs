use std::fs::remove_dir_all;
use std::{fs::read_dir, path::PathBuf, sync::Arc};

use fs_extra::dir::get_size;
use lazy_static::lazy_static;
use serenity::model::prelude::*;
use serenity::utils::Color;
use serenity::{framework::standard::CommandResult, http::Http};
use ytd_rs::{ResultType, YoutubeDL};

lazy_static! {
    pub static ref MAX_DISCORD_FILE_SIZE: u64 = 8_000_000; // 8mb
    pub static ref MAX_FILE_SIZE: u64 = 200_000_000; // 200mb
}

pub async fn start_download(msg: Message, id: u64, http: Arc<Http>, url: String) -> CommandResult {
    // create the download directory
    let dir = create_download_dir(id).await?;
    // check if download directory is empty
    if let Ok(read) = read_dir(&dir) {
        // if not, download is running
        if read.count() != 0 {
            // we don't want the directory to be cleaned
            // because a download is running
            return send_error(&msg, &http, "download running").await;
        }
    }

    // create an update message to inform user about the current download state
    let mut update_message = msg
        .channel_id
        .send_message(&http, |m| m.content("Starting download ..."))
        .await?;

    // download the video
    let file = match make_download(&dir, &url).await {
        Err(why) => {
            remove_dir_all(&dir)?; // clean dir on error
            return send_error(&msg, &http, &why).await;
        }
        Ok(path) => path,
    };

    // get size of the file
    let size = match get_size(file.as_path()) {
        Ok(size) => size,
        Err(why) => {
            remove_dir_all(&dir)?; // clean dir on error
            return send_error(&msg, &http, &format!("{}", why)).await;
        }
    };

    // sizes smaller than 8mb can be uploaded to discord directly
    if size < *MAX_FILE_SIZE && size < *MAX_DISCORD_FILE_SIZE {
        update_message
            .edit(&http, |m| m.content("Uploading to Discord ..."))
            .await?;
        send_file_to_channel(file, &msg, &http).await?;
        // if file is below the setted limit but above the 8mb we can upload it to transfer.sh
    } else if size < *MAX_FILE_SIZE {
        update_message
            .edit(&http, |m| m.content("Uploading to transfer.sh ..."))
            .await?;
        send_file_to_transfersh(file, &msg, &http, &id.to_string()).await?;
        // else we have to inform the user that the file was too chonky
    } else {
        let max_mb = *MAX_FILE_SIZE / 1_000_000;
        send_error(
            &msg,
            &http,
            &format!("Your download was larger than {}mb", max_mb),
        )
        .await?;
    }

    // finally clear everything
    // to be ready for the next download
    remove_dir_all(&dir)?;

    Ok(())
}

async fn make_download(dir: &PathBuf, url: &str) -> Result<PathBuf, String> {
    // get the youtubedl task
    let ytd: YoutubeDL = match dir.to_str() {
        Some(path) => YoutubeDL::new(path, vec![], url)?,
        None => return Err("couldn't get directory for download".to_string()),
    };

    // get the downloaded file
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
    // start download
    let result = ytd.download();

    // check output
    let path = match result.result_type() {
        ResultType::SUCCESS => result.output_dir(),
        ResultType::IOERROR | ResultType::FAILURE => {
            // return if error
            return Err(format!("Couldn't download {}", url));
        }
    };

    // read dir
    let dir_entry = match read_dir(path.as_path()) {
        Ok(read) => read,
        Err(_) => {
            return Err("couldn't read download directory".to_string());
        }
    };

    for entry in dir_entry {
        if let Ok(entry) = entry {
            let path = entry.path();
            // just return the first file that we'll find
            if path.is_file() {
                return Ok(path);
            }
        }
    }

    // if no file was found, return error
    Err("Couldn't find downloaded file".to_string())
}

async fn send_file_to_channel(file: PathBuf, msg: &Message, http: &Arc<Http>) -> CommandResult {
    // send files to discord
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
    // upload via transfer.sh
    let link = crate::model::upload_file(&file, safe_name)?;
    // send user the output (link/error)
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
