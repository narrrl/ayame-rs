use chrono::Weekday;
use core::fmt;
use mensa_swfr_rs::mensa;
use regex::Regex;
use std::{fmt::Display, path::PathBuf};

use poise::{serenity::Result as SerenityResult, serenity_prelude::CreateEmbed};
use rand::Rng;

use tracing::error;

use crate::commands::manage::get_bound_channel_id;
use crate::error::*;
use crate::{Context, Error};

pub fn bot_dir() -> PathBuf {
    let mut dir = std::env::current_exe().expect("couldn't get bot directory");
    dir.pop();
    dir
}

pub fn get_file(name: &str) -> PathBuf {
    let mut dir = bot_dir();
    dir.push(name);
    dir
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

pub fn create_mensa_plan_by_day(
    day: &mensa::Day,
) -> Result<CreateEmbed, Box<dyn std::error::Error>> {
    let mut embed = CreateEmbed::default();
    embed.title(format!(
        "{} ({})",
        weekday_german(&day.weekday()?),
        day.to_chrono()?.format("%d.%m.%Y")
    ));
    for menu in day.menues.iter() {
        let price = &menu.price;
        embed.field(
            &menu.art,
            format!(
                "{}\n\nZusatz: {}\n\nPreis: {}/{}/{}",
                Regex::new(r"--+").unwrap().replace(&menu.name, "\n\n!!! "),
                match &menu.food_type {
                    Some(typ) => typ,
                    None => "None",
                },
                price.price_students,
                price.price_workers,
                price.price_guests,
            ),
            false,
        );
    }
    Ok(embed)
}

pub fn translate_weekday(wd: &str) -> String {
    match wd.to_lowercase().as_str() {
        "montag" | "mo" => format!("{}", Weekday::Mon),
        "dienstag" | "di" => format!("{}", Weekday::Tue),
        "mittwoch" | "mi" => format!("{}", Weekday::Wed),
        "donnerstag" | "do" => format!("{}", Weekday::Thu),
        "freitag" | "fr" => format!("{}", Weekday::Fri),
        "samstag" | "sa" => format!("{}", Weekday::Sat),
        "sonntag" | "so" => format!("{}", Weekday::Sun),
        _ => String::from(wd),
    }
}

pub fn weekday_german(wd: &Weekday) -> String {
    match wd {
        Weekday::Mon => String::from("Montag"),
        Weekday::Tue => String::from("Dienstag"),
        Weekday::Wed => String::from("Mittwoch"),
        Weekday::Thu => String::from("Donnerstag"),
        Weekday::Fri => String::from("Freitag"),
        Weekday::Sat => String::from("Samstag"),
        Weekday::Sun => String::from("Sonntag"),
    }
}

pub fn check_result<T>(result: SerenityResult<T>) {
    if let Err(why) = result {
        error!("error: {:?}", why);
    }
}

pub fn check_result_ayame<T>(result: Result<T, Error>) {
    if let Err(why) = result {
        error!("error: {:?}", why);
    }
}

pub(crate) async fn guild_only(ctx: Context<'_>) -> Result<bool, Error> {
    match ctx.guild() {
        Some(_) => Ok(true),
        None => Err(Error::Input(NOT_IN_GUILD)),
    }
}

pub(crate) async fn bind_command(ctx: Context<'_>) -> Result<bool, Error> {
    let guild_id = match ctx.guild_id() {
        Some(id) => id.0 as i64,
        None => return Err(Error::Input(NOT_IN_GUILD)),
    };
    match get_bound_channel_id(&ctx.data().database, guild_id).await? {
        Some(bind_id) => {
            if bind_id == ctx.channel_id().0 {
                // looks dumb, but i want to send error
                // as explenation when
                // user doesn't do stuff right
                Ok(true)
            } else {
                Err(Error::Input(ONLY_IN_BOT_CHANNEL))
            }
        }
        None => Err(Error::Input(NO_BOT_CHANNEL)),
    }
}

pub struct Bar {
    pub pos_icon: String,

    pub line_icon: String,

    pub length: usize,

    pub pos: f64,
}

impl Default for Bar {
    fn default() -> Bar {
        Bar {
            pos_icon: ">".to_string(),
            line_icon: "=".to_string(),
            length: 30,
            pos: 0.0f64,
        }
    }
}

impl Bar {
    pub fn set<'a>(&'a mut self, pos: f64) -> &'a mut Bar {
        self.pos = pos;
        self
    }

    pub fn set_len<'a>(&'a mut self, len: usize) -> &'a mut Bar {
        self.length = len;
        self
    }
}

impl Display for Bar {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let r: usize = (self.pos * self.length as f64) as usize;
        let prev = self.line_icon.repeat(r);
        let (h, next) = if r >= self.length {
            (String::new(), String::new())
        } else {
            (
                String::from(&self.pos_icon),
                self.line_icon.repeat(self.length - r - 1),
            )
        };
        write!(fmt, "[{}{}{}]", prev, h, next)
    }
}
