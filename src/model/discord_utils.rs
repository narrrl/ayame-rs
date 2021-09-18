use serenity::{builder::CreateEmbed, utils::Color};

pub fn default_embed() -> CreateEmbed {
    let mut e = CreateEmbed::default();
    set_defaults_for_embed(&mut e);
    e
}

pub fn set_defaults_for_embed(e: &mut CreateEmbed) {
    e.color(Color::from_rgb(238, 14, 97));
    let time = chrono::Utc::now();
    e.timestamp(&time);
    // add more defaults for embeds ...
}

pub fn set_defaults_for_error(e: &mut CreateEmbed, message: &str) {
    // set defaults
    set_defaults_for_embed(e);
    // red color to indicate error
    e.color(Color::from_rgb(204, 0, 0));
    e.title(format!("Error: {}", message));
}
