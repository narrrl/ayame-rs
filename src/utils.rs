use mensa_swfr_rs::mensa;
use std::path::PathBuf;

use poise::serenity_prelude::CreateEmbed;
use rand::Rng;

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
    embed.title(day.weekday()?);
    for menu in day.menues.iter() {
        let price = &menu.price;
        embed.field(
            &menu.name,
            format!(
                "Kind:               {}
        Type:               {}
        Students:           {}
        Workers:            {}
        Guests:             {}
        Students (School):  {}",
                menu.art,
                match menu.food_type {
                    Some(typ) => typ,
                    None => "None",
                },
                price.price_students,
                price.price_workers,
                price.price_guests,
                price.price_school
            ),
            false,
        );
    }
    Ok(embed)
}
