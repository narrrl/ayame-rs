pub mod discord_commands;
pub mod image_processing;
pub mod youtubedl;

use rand::Rng;
use std::{
    fmt,
    fs::canonicalize,
    io::Error,
    path::PathBuf,
    process::{Command, Output, Stdio},
};

pub fn upload_file(file: &PathBuf, safe_name: &str) -> Result<String, String> {
    // canonicalize path to file for a absolute path
    let file = match canonicalize(&file) {
        Ok(path) => path,
        Err(why) => {
            return Err(format!("Couldn't canonicalize path to file, {:?}", why));
        }
    };

    // don't upload a directory
    if file.is_dir() {
        return Err("Can't upload a directory".to_string());
    }

    // get the file extension
    let extension = match file.extension() {
        Some(name) => match name.to_str() {
            Some(name) => name,
            None => {
                return Err(format!("Couldn't get file name of {:?}", file));
            }
        },
        None => {
            return Err(format!("Couldn't get file name of {:?}", file));
        }
    };

    // get absolute file path as string
    let file = match file.to_str() {
        Some(path) => path,
        None => {
            return Err(format!("Couldn't convert {:?} to a string", file));
        }
    };

    // run upload
    let output = match run_upload(file, safe_name, extension) {
        Err(why) => {
            return Err(format!("Couldn't upload file, {:?}", why));
        }
        Ok(output) => output,
    };

    // return output of curl
    let output = String::from_utf8(output.stdout).expect("Couldn't convert output of curl");

    Ok(output)
}

fn run_upload(file: &str, file_name: &str, extension: &str) -> Result<Output, Error> {
    // create process
    let mut cmd = Command::new("curl");

    // set args and env
    cmd.env("LC_ALL", "en_US.UTF-8")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg("-H")
        .arg("Max-Days: 1")
        .arg("--upload-file")
        .arg(file)
        .arg(format!("http://transfer.sh/{}.{}", file_name, extension));

    // start
    match cmd.spawn() {
        Err(why) => {
            return Err(why);
        }
        Ok(process) => process,
    }
    .wait_with_output() // get output
}

pub fn mock_text(text: &str) -> String {
    let mut mock_str = String::new();

    let mut rng = rand::thread_rng();

    for ch in text.chars() {
        if rng.gen() {
            mock_str.push_str(&ch.to_uppercase().collect::<String>());
        } else {
            mock_str.push_str(&ch.to_lowercase().collect::<String>());
        }
    }
    mock_str
}

pub struct Timestamp {
    h: u32,
    m: u32,
    s: u32,
    ms: u32,
}

impl Timestamp {
    #[allow(dead_code)]
    pub fn from(h: u32, m: u32, s: u32, ms: u32) -> Result<Timestamp, Box<dyn std::error::Error>> {
        if m > 60 {
            return Err(Box::from(TimeFormatError {
                msg: format!("Expected minutes amount below 60, got {}", &m),
            }));
        } else if s > 60 {
            return Err(Box::from(TimeFormatError {
                msg: format!("Expected seconds amount below 60, got {}", &s),
            }));
        } else if ms > 1000 {
            return Err(Box::from(TimeFormatError {
                msg: format!("Expected milliseconds amount below 1000, got {}", &ms),
            }));
        }
        Ok(Timestamp { h, m, s, ms })
    }

    #[allow(dead_code)]
    pub fn from_secs_with_ms(s: u32, ms: u32) -> Result<Timestamp, Box<dyn std::error::Error>> {
        let m = s / 60 - s / 3600;
        let h = s / 3600;
        Timestamp::from(h, m, s - (h * 3600 + m * 60), ms)
    }

    #[allow(dead_code)]
    pub fn from_secs(s: u32) -> Result<Timestamp, Box<dyn std::error::Error>> {
        Timestamp::from_secs_with_ms(s, 0)
    }

    #[allow(dead_code)]
    pub fn from_string(input: &str) -> Result<Timestamp, Box<dyn std::error::Error>> {
        let mut split = input.split(".").collect::<Vec<&str>>();
        if split.len() == 1 {
            return Timestamp::from_string_without_ms(input);
        }
        let hms = split.remove(0);
        let ms = split.remove(0);
        for s in hms.split(":").collect::<Vec<&str>>().iter() {
            split.push(s);
        }
        split.push(ms);
        let (h, m, s, ms) = match split.len() {
            4 => (
                split.get(0).unwrap().parse::<u32>()?,
                split.get(1).unwrap().parse::<u32>()?,
                split.get(2).unwrap().parse::<u32>()?,
                split.get(3).unwrap().parse::<u32>()?,
            ),
            3 => (
                0,
                split.get(0).unwrap().parse::<u32>()?,
                split.get(1).unwrap().parse::<u32>()?,
                split.get(2).unwrap().parse::<u32>()?,
            ),
            2 => (
                0,
                0,
                split.get(0).unwrap().parse::<u32>()?,
                split.get(1).unwrap().parse::<u32>()?,
            ),
            1 => (0, 0, 0, split.get(0).unwrap().parse::<u32>()?),
            _ => {
                return Err(Box::from(TimeFormatError {
                    msg: "Invalid amount of arguments".to_string(),
                }));
            }
        };

        Timestamp::from_secs_with_ms(h * 3600 + m * 60 + s, ms)
    }

    fn from_string_without_ms(input: &str) -> Result<Timestamp, Box<dyn std::error::Error>> {
        let split = input.split(":").collect::<Vec<&str>>();
        let (h, m, s) = match split.len() {
            3 => (
                split.get(0).unwrap().parse::<u32>()?,
                split.get(1).unwrap().parse::<u32>()?,
                split.get(2).unwrap().parse::<u32>()?,
            ),
            2 => (
                0,
                split.get(0).unwrap().parse::<u32>()?,
                split.get(1).unwrap().parse::<u32>()?,
            ),
            1 => (0, 0, split.get(0).unwrap().parse::<u32>()?),
            _ => {
                return Err(Box::from(TimeFormatError {
                    msg: "Invalid amount of arguments".to_string(),
                }));
            }
        };

        Timestamp::from_secs(h * 3600 + m * 60 + s)
    }

    #[allow(dead_code)]
    pub fn seconds(&self) -> u32 {
        self.s
    }

    #[allow(dead_code)]
    pub fn in_seconds(&self) -> u32 {
        self.s + self.m * 60 + self.h * 3600
    }

    #[allow(dead_code)]
    pub fn hours(&self) -> u32 {
        self.h
    }

    #[allow(dead_code)]
    pub fn minutes(&self) -> u32 {
        self.m
    }

    #[allow(dead_code)]
    pub fn milliseconds(&self) -> u32 {
        self.ms
    }
}

#[derive(Debug, Clone)]
pub struct TimeFormatError {
    msg: String,
}

impl std::fmt::Display for TimeFormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for TimeFormatError {
    fn description(&self) -> &str {
        self.msg.as_ref()
    }
}

impl std::string::ToString for Timestamp {
    fn to_string(&self) -> String {
        format!(
            "{:0>2}:{:0>2}:{:0>2}.{:0>2}",
            self.h, self.m, self.s, self.ms
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::model::Timestamp;
    #[test]
    fn some_tests() -> Result<(), Box<dyn std::error::Error>> {
        let stamp = Timestamp::from_string("00:12:15.00")?;
        assert_eq!(stamp.h, 0);
        assert_eq!(stamp.m, 12);
        assert_eq!(stamp.s, 15);
        assert_eq!(stamp.ms, 0);
        assert_eq!(stamp.to_string(), "00:12:15.00".to_string());
        let stamp = Timestamp::from_string("12:15")?;
        assert_eq!(stamp.h, 0);
        assert_eq!(stamp.m, 12);
        assert_eq!(stamp.s, 15);
        assert_eq!(stamp.ms, 0);
        assert_eq!(stamp.to_string(), "00:12:15.00".to_string());
        let stamp = Timestamp::from_string("735")?;
        assert_eq!(stamp.h, 0);
        assert_eq!(stamp.m, 12);
        assert_eq!(stamp.s, 15);
        assert_eq!(stamp.ms, 0);
        assert_eq!(stamp.to_string(), "00:12:15.00".to_string());
        let stamp = Timestamp::from_string("05.50")?;
        assert_eq!(stamp.h, 0);
        assert_eq!(stamp.m, 0);
        assert_eq!(stamp.s, 5);
        assert_eq!(stamp.ms, 50);
        assert_eq!(stamp.to_string(), "00:00:05.50".to_string());
        Ok(())
    }

    #[test]
    fn some_more_tests() -> Result<(), Box<dyn std::error::Error>> {
        let stamp = Timestamp::from(0, 12, 15, 0)?;
        assert_eq!(stamp.h, 0);
        assert_eq!(stamp.m, 12);
        assert_eq!(stamp.s, 15);
        assert_eq!(stamp.ms, 0);
        assert_eq!(stamp.to_string(), "00:12:15.00".to_string());
        Ok(())
    }
}
