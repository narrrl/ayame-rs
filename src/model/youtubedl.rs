use std::fs::remove_dir_all;
use std::{fs::read_dir, path::PathBuf, sync::Arc};

use fs_extra::dir::get_size;
use serenity::model::prelude::*;
use serenity::utils::Color;
use serenity::{
    framework::standard::{CommandError, CommandResult},
    http::Http,
};
use tracing::error;
use ytd_rs::{Arg, ResultType, YoutubeDL};

pub const MAX_DISCORD_FILE_SIZE: u64 = 8_000_000; // 8mb
pub const MAX_FILE_SIZE: u64 = 200_000_000; // 200mb

pub struct YTDL {
    channel: ChannelId,
    author_id: u64,
    http: Arc<Http>,
    args: Vec<Arg>,
}

impl YTDL {
    pub fn new(channel: ChannelId, author_id: u64, http: Arc<Http>) -> YTDL {
        YTDL {
            channel,
            author_id,
            http,
            args: Vec::new(),
        }
    }

    ///
    /// Set default options to convert the file to a mp3.
    /// Also embeds the thumbnail as cover art
    ///
    pub fn set_audio_only<'a>(&'a mut self) -> &'a mut YTDL {
        self.args.push(Arg::new("--extract-audio"));
        self.args.push(Arg::new_with_arg("--audio-format", "mp3"));
        self.args.push(Arg::new("--embed-thumbnail"));
        self
    }

    ///
    /// Sets some nice defaults to have a better youtubedl experience.
    ///
    /// Adds an age limit to bypass age restrictions
    /// Adds an output size limit because especially twitter videos keeps exploading
    /// Adds metadata to the file
    ///
    pub fn set_defaults<'a>(&'a mut self) -> &'a mut YTDL {
        self.args.push(Arg::new_with_arg("--age-limit", "69"));
        self.args
            .push(Arg::new_with_arg("--output", "%(title).90s.%(ext)s"));
        self.args.push(Arg::new("--add-metadata"));
        self
    }
    #[allow(dead_code)]
    pub fn arg<'a>(&'a mut self, arg: Arg) -> &'a mut YTDL {
        self.args.push(arg);
        self
    }
    #[allow(dead_code)]
    pub fn args<'a>(&'a mut self, args: &Vec<Arg>) -> &'a mut YTDL {
        for arg in args.iter() {
            self.args.push(arg.clone());
        }
        self
    }

    ///
    /// Starts the YoutubeDL download and sends it to the user.
    ///
    /// It also checks if the user is already downloading and if the file is too chonky
    ///
    pub async fn start_download(&self, url: String) -> CommandResult {
        // create the download directory
        let dir = match self.get_download_directory().await {
            Ok(dir) => dir,
            Err(why) => return self.send_error(&why).await,
        };

        // create an update message to inform user about the current download state
        let mut update_message = self
            .channel
            .send_message(&self.http, |m| m.content("Starting download ..."))
            .await?;

        // download the video
        let file = match self.download_file(&dir, &url).await {
            Err(why) => {
                remove_dir_all(&dir)?; // clean dir on error
                return self.send_error(&why).await;
            }
            Ok(path) => path,
        };

        if let Err(why) = self.upload_file(&mut update_message, file).await {
            remove_dir_all(&dir)?;
            self.send_error(&format!("{}", why)).await?;
        }

        // finally clear everything
        remove_dir_all(&dir)?;

        Ok(())
    }

    ///
    /// gets the download directory, but returns an error if it exists
    /// because then the user is already downloading something
    ///
    async fn get_download_directory(&self) -> Result<PathBuf, String> {
        // tmp download directory is
        // {bot_dir}/tmp/ytd/id
        let mut dir = crate::BOT_DIR.clone();
        dir.push("tmp");
        dir.push("ytd");
        dir.push(format!("{}", &self.author_id));

        if dir.exists() {
            return Err("Download running!".to_string());
        }
        Ok(dir)
    }

    ///  creates the 'YoutubeDL' and runs it, then returns the first file that it finds
    async fn download_file(&self, dir: &PathBuf, url: &str) -> Result<PathBuf, String> {
        // get the youtubedl task
        let ytd = match YoutubeDL::new(dir, self.args.clone(), url) {
            Ok(ytd) => ytd,
            Err(why) => return Err(why.to_string()),
        };

        let download_dir = self.run_youtubedl(&ytd, url).await?;

        // get the downloaded file
        let file = self.get_file(&download_dir).await?;

        Ok(file)
    }

    ///
    /// runs the youtubedl thread and prints eny error via the logger
    ///
    async fn run_youtubedl(&self, ytd: &YoutubeDL, url: &str) -> Result<PathBuf, String> {
        // start download
        let result = ytd.download();

        // check output
        match result.result_type() {
            ResultType::SUCCESS => Ok(result.output_dir().clone()),
            ResultType::IOERROR | ResultType::FAILURE => {
                error!(
                    "YoutubeDL exited with error: {:?}",
                    result
                        .output()
                        .replace("Usage: youtube-dl [OPTIONS] URL [URL...]\\n\\n", "")
                );
                // return if error
                return Err(format!("Couldn't download {}", url));
            }
        }
    }

    ///
    /// returns the file that was downloaded
    ///
    async fn get_file(&self, download_dir: &PathBuf) -> Result<PathBuf, String> {
        // read dir
        let dir_entry = match read_dir(download_dir.as_path()) {
            Ok(read) => read,
            Err(_) => {
                return Err("couldn't read download directory".to_string());
            }
        };

        for entry in dir_entry {
            if let Ok(entry) = entry {
                let path = entry.path();
                // just return the first file that we find
                if path.is_file() {
                    return Ok(path);
                }
            }
        }

        // if no file was found, return error
        Err("Couldn't find downloaded file".to_string())
    }

    ///
    ///  Uploads the file to either discord or transfer.sh depending on the file size.
    ///
    ///  Returns an error if the file is way to chonky
    ///
    async fn upload_file(&self, update_message: &mut Message, file: PathBuf) -> CommandResult {
        // get size of the file
        let size = get_size(file.as_path())?;

        // sizes smaller than 8mb can be uploaded to discord directly
        if size < MAX_FILE_SIZE && size < MAX_DISCORD_FILE_SIZE {
            update_message
                .edit(&self.http, |m| m.content("Uploading to Discord ..."))
                .await?;
            self.send_file_to_channel(file).await?;
            // if file is below the setted limit but above the 8mb we can upload it to transfer.sh
        } else if size < MAX_FILE_SIZE {
            update_message
                .edit(&self.http, |m| m.content("Uploading to transfer.sh ..."))
                .await?;
            self.send_file_to_transfersh(&file).await?;
            // else we have to inform the user that the file was too chonky
        } else {
            let max_mb = MAX_FILE_SIZE / 1_000_000;
            return Err(CommandError::from(format!(
                "Your download was larger than {}mb",
                max_mb
            )));
        }

        update_message
            .edit(&self.http, |m| m.content("Done!"))
            .await?;
        Ok(())
    }

    async fn send_file_to_channel(&self, file: PathBuf) -> CommandResult {
        // send files to discord
        self.channel
            .send_files(&self.http, &vec![file], |m| m.content(""))
            .await?;
        Ok(())
    }

    async fn send_file_to_transfersh(&self, file: &PathBuf) -> CommandResult {
        // upload via transfer.sh
        let output = crate::model::upload_file(file, &self.author_id.to_string())?;
        // send user the output (link/error)
        self.channel
            .send_message(&self.http, |m| m.content(output))
            .await?;
        Ok(())
    }

    async fn send_error(&self, error_msg: &str) -> CommandResult {
        self.channel
            .send_message(&self.http, |m| {
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
}
