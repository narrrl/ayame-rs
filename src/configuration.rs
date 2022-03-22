use std::fs::File;
use std::io::prelude::*;

use poise::serenity_prelude::Color;
use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    token: String,
    prefix: String,
    application_id: u64,
    copy_codec: Option<bool>,
    youtube_api_key: String,
    mensa_api_key: Option<String>,
    color: String,
}

impl Config {
    pub fn token<'a>(&'a self) -> &'a String {
        &self.token
    }

    pub fn prefix<'a>(&'a self) -> &'a String {
        &self.prefix
    }

    #[allow(dead_code)]
    pub fn get_application_id<'a>(&'a self) -> &'a u64 {
        &self.application_id
    }

    #[allow(dead_code)]
    pub fn copy_codec(&self) -> bool {
        if let Some(b) = self.copy_codec {
            b
        } else {
            false
        }
    }

    pub fn mensa_api_key<'a>(&'a self) -> &'a Option<String> {
        &self.mensa_api_key
    }

    #[allow(dead_code)]
    pub fn youtube_api_key<'a>(&'a self) -> &'a String {
        &self.youtube_api_key
    }

    pub fn color<'a>(&'a self) -> Result<Color, Error> {
        Ok(Color::from(
            u32::from_str_radix(self.color.as_str(), 16)
                .map_err(|_| Error::Failure("couldn't parse color"))?,
        ))
    }
}

pub fn config() -> Config {
    let config_path = crate::utils::get_file("config.toml");

    let mut file = File::open(&config_path).expect("Couldn't open config.toml");

    let mut config_content = String::new();

    file.read_to_string(&mut config_content)
        .expect("Couldn't read config.toml");

    // check if config is deserializable, else try again
    toml::from_str(&config_content).expect("couldn't deserialize config")
}
