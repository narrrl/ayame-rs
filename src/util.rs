use apex_rs::model::Map;
use poise::serenity_prelude::{self as serenity, CreateEmbed};

pub fn type_of<T>(_: T) -> &'static str {
    std::any::type_name::<T>()
}

pub fn embed_map(map: &Map) -> serenity::CreateEmbed {
    let mut embed = CreateEmbed::default();
    let time_left = map.end_as_date() - chrono::Utc::now();
    embed
        .field(
            map.name(),
            format!(
                "from {} to {}",
                map.start_as_date().format("%H:%M"),
                map.end_as_date().format("%H:%M")
            ),
            true,
        )
        .field(
            "Time left",
            time_left.num_minutes().to_string()
                + " minutes "
                + &(time_left.num_seconds() % 60).to_string()
                + " seconds",
            true,
        )
        .color(crate::color());
    if let Some(url) = map.asset() {
        embed.image(url);
    }
    embed
}
