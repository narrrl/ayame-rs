use std::io::prelude::*;
use std::{fs::File, path::PathBuf};

use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    token: String,
    prefix: String,
}

impl Config {
    pub fn token(&self) -> String {
        self.token.clone()
    }

    pub fn prefix(&self) -> String {
        self.prefix.clone()
    }
}

fn get_config(interactively: bool) -> Config {
    let config_path = crate::get_file("config.toml");

    // create config if it doesn't exist
    if !config_path.exists() && interactively {
        return reset_config(&config_path);
    }
    let mut file = File::open(&config_path).expect("Couldn't open config.toml");

    let mut config_content = String::new();

    file.read_to_string(&mut config_content)
        .expect("Couldn't read config.toml");

    // check if config is deserializable, else try again
    match toml::from_str(&config_content) {
        Ok(config) => config,
        Err(_) => {
            if interactively {
                return reset_config(&config_path);
            } else {
                panic!("Couldn't parse config file");
            }
        }
    }
}

///
/// Gets the bot configuration
/// Creates a config file if it doesn't exists
///
pub fn create_config_interactive() -> Config {
    get_config(true)
}

pub fn config() -> Config {
    get_config(false)
}

///
/// Creates the configuration interactively.
/// The user puts in the token first
/// then the prefix
///
fn get_config_from_user() -> Config {
    info!("Put in your bot token:");
    let token: String = get_userinput()
        .expect("Couldn't read your input")
        // trim because spaces or linebreaks would break everything
        .trim()
        .to_string();

    info!("Put in your bot prefix:");
    let prefix: String = get_userinput()
        .expect("Couldn't read your input")
        // trim because spaces or linebreaks would break everything
        .trim()
        .to_string();

    Config { token, prefix }
}

///
/// Simply take the next line of user input
///
fn get_userinput() -> std::io::Result<String> {
    let mut input = String::new();

    std::io::stdin().read_line(&mut input)?;

    Ok(input)
}

///
/// Create config (overwrites existing) again.
/// Basically takes the users input and writes into the config file
///
fn reset_config(config_path: &PathBuf) -> Config {
    let mut file = File::create(&config_path).expect("Couldn't create config.toml");

    let config = get_config_from_user();

    let config_content =
        toml::ser::to_string(&config).expect("Couldn't create config, create one yourself");

    file.write_all(config_content.as_bytes())
        .expect("Couldn't write to config.toml");
    config
}
