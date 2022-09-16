use apex_rs::model::Map;
use chrono::{DateTime, Utc};
use poise::serenity_prelude::{self as serenity, CreateEmbed};

pub fn type_of<T>(_: T) -> &'static str {
    std::any::type_name::<T>()
}

pub fn embed_map(map: &Map, next_map: bool) -> serenity::CreateEmbed {
    let mut embed = CreateEmbed::default();
    embed.field(
        map.name(),
        format!(
            "from {} to {}",
            to_short_timestamp(map.start_as_date()),
            to_short_timestamp(map.end_as_date()),
        ),
        true,
    );
    let (text, time) = if next_map {
        ("", map.start_as_date())
    } else {
        ("next map", map.end_as_date())
    };
    embed
        .field(
            "Time",
            format!("{} {}", text, to_relative_timestamp(&time)),
            true,
        )
        .color(crate::color());
    if let Some(url) = map.asset() {
        embed.image(url);
    }
    embed
}

pub fn to_relative_timestamp(date: &DateTime<Utc>) -> String {
    let unix_time = date.timestamp();
    format!("<t:{}:R>", unix_time)
}
pub fn to_short_timestamp(date: DateTime<Utc>) -> String {
    let unix_time = date.timestamp();
    format!("<t:{}:t>", unix_time)
}
