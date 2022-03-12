use chrono::Weekday;
use mensa_swfr_rs::mensa;
use regex::Regex;
use std::path::PathBuf;

use poise::{serenity::Result as SerenityResult, serenity_prelude::CreateEmbed};
use rand::Rng;

use tracing::error;

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
