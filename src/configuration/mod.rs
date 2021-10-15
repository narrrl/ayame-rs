use std::fs::File;
use std::io::prelude::*;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    token: String,
    prefix: String,
    application_id: u64,
    copy_codec: Option<bool>,
    youtube_api_key: String,
}

impl Config {
    pub fn token(&self) -> String {
        self.token.clone()
    }

    pub fn prefix(&self) -> String {
        self.prefix.clone()
    }

    pub fn get_application_id(&self) -> u64 {
        self.application_id.clone()
    }

    pub fn copy_codec(&self) -> bool {
        if let Some(b) = self.copy_codec {
            b
        } else {
            false
        }
    }

    pub fn youtube_api_key(&self) -> String {
        self.youtube_api_key.clone()
    }
}

pub fn config() -> Config {
    let config_path = crate::get_file("config.toml");

    let mut file = File::open(&config_path).expect("Couldn't open config.toml");

    let mut config_content = String::new();

    file.read_to_string(&mut config_content)
        .expect("Couldn't read config.toml");

    // check if config is deserializable, else try again
    toml::from_str(&config_content).expect("couldn't deserialize config")
}
