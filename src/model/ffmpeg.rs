use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::Timestamp;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + std::marker::Send>>;

pub struct FFmpeg {
    in_file: String,
    interval: Option<(Timestamp, Option<Timestamp>)>,
}

#[derive(Debug)]
pub struct FFmpegError {
    description: String,
}

impl FFmpegError {
    pub fn new(description: &str) -> FFmpegError {
        FFmpegError {
            description: description.to_string(),
        }
    }
}

impl std::fmt::Display for FFmpegError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl std::error::Error for FFmpegError {
    fn description(&self) -> &str {
        self.description.as_ref()
    }
}

unsafe impl std::marker::Send for FFmpegError {}

impl FFmpeg {
    pub fn new(in_file: &PathBuf) -> Result<FFmpeg> {
        let in_str = match in_file.to_str() {
            Some(str) => str,
            None => {
                return Err(Box::new(FFmpegError::new(&format!(
                    "couldn't convert path of input file {:#?}",
                    &in_file
                ))));
            }
        }
        .to_string();

        Ok(FFmpeg {
            in_file: in_str,
            interval: None,
        })
    }

    pub fn set_interval<'a>(
        &'a mut self,
        start: Timestamp,
        end: Option<Timestamp>,
    ) -> &'a mut FFmpeg {
        self.interval = Some((start, end));
        self
    }

    pub fn cut_file(&self, out: &mut PathBuf, copy_streams: bool) -> Result<()> {
        // create ffmpeg command
        let mut cmd = Command::new("ffmpeg");
        cmd.env("LC_ALL", "en_US.UTF-8")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // input file as first arg
        cmd.arg("-i").arg(&self.in_file);

        // set interval if set
        if let Some((start, end)) = &self.interval {
            // from start
            cmd.arg("-ss").arg(&start.to_string());
            // to end, if set
            if let Some(end) = end {
                cmd.arg("-to").arg(&end.to_string());
            }
        }

        // set copy streams trait for faster cutting
        if copy_streams {
            cmd.arg("-c").arg("copy");
        }

        // get path to output file
        let out_path = match out.to_str() {
            Some(str) => str,
            None => {
                return Err(Box::new(FFmpegError::new(&format!(
                    "couldn't get path from output file {:#?}",
                    out
                ))));
            }
        };

        // set output path as last argument
        cmd.arg(out_path);

        // spawn process
        let process = match cmd.spawn() {
            Ok(process) => process,
            Err(_) => {
                return Err(Box::new(FFmpegError::new(
                    "couldn't spawn ffmpeg process, is ffmpeg installed?",
                )));
            }
        };

        // wait to finish
        let out = process.wait_with_output();
        // check for errors
        if let Err(why) = out {
            return Err(Box::new(FFmpegError::new(&format!(
                "An error happend while spawning ffmgep {:#?}",
                why
            ))));
        }

        // done
        Ok(())
    }
}
