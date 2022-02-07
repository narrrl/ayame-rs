use crate::model::discord_utils::check_msg;
use crate::model::image_processing;
use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, File};
use std::io::Write;

use lazy_static::lazy_static;
use regex::Regex;
use serenity::framework::standard::Args;
use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::utils::read_image;

lazy_static! {
    pub static ref IMAGE_REGEX: Regex =
        Regex::new(r".+\.(gif|png|jpg|jpeg)").expect("Couldn't create image regex");
}
pub const MAX_EMOTE_SIZE: u64 = 256_000; // kb

#[command]
#[only_in(guilds)]
#[required_permissions("MANAGE_EMOJIS")]
#[num_args(1)]
#[usage("[emote_name]")]
#[description(
    "Uploads the attached image as emote with the given `emote_name`.
    Resizes images until the image is small enough."
)]
async fn addemote(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    // get emote name
    let emote_name = match args.current() {
        Some(arg) => arg,
        None => return Ok(()),
    };

    // check if emote name is long enough
    if emote_name.len() < 2 {
        msg.reply(&ctx.http, "Emote name must be atleast 2 characters long!")
            .await?;
        return Ok(());
    }

    // create download directory to safe the attached image

    // get the bot directory
    let mut path = crate::BOT_DIR.clone();
    // tmp is the default temporary directory for the bot
    path.push("tmp");
    // create a subfolder with the authors id (to prevent name collisions)
    path.push(msg.author.id.to_string());

    // directory should be created,
    // if error something went really wrong. bot owner should fix it.
    create_dir_all(&path)?;

    // get the first attachment
    let img = match msg.attachments.first() {
        Some(img) => img,
        None => return Ok(()),
    };

    // return if user is too stupid to attach an image
    if !IMAGE_REGEX.is_match(&img.filename) {
        return Ok(());
    }

    path.push(&img.filename);

    let mut buf = File::create(&path)?;

    // get image as byte array
    let content = match img.download().await {
        Ok(content) => content,
        Err(_) => {
            msg.reply(&ctx.http, "Couldn't download image!").await?;

            return Ok(());
        }
    };

    // check if image succeedes the size limit

    // write image to disk
    buf.write_all(&content)?;

    if content.len() > MAX_EMOTE_SIZE as usize {
        match image_processing::reduce_emote_size(&path) {
            Ok(p) => p,
            Err(_) => {
                msg.reply(&ctx.http, "Image was to chonky").await?;
                return Ok(());
            }
        };
    }

    let guild = match msg.guild(&ctx.cache).await {
        Some(guild) => guild,
        None => return Ok(()),
    };

    // upload emote
    let emote = guild
        .create_emoji(&ctx.http, emote_name, &(read_image(&path)?))
        .await?;

    msg.react(&ctx.http, emote).await?;

    Ok(())
}

#[command("delemote")]
#[only_in(guilds)]
#[required_permissions("MANAGE_EMOJIS")]
#[num_args(1)]
#[usage("[emote_name]")]
#[description("Removes all occurencies of emotes with that name from that guild")]
async fn delete_emote(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let emote_name = match args.current() {
        Some(arg) => arg,
        None => return Ok(()),
    };
    let mut emotes = guild.emojis(&ctx.http).await?;
    emotes.retain(|e| e.name == emote_name);
    for emote in emotes.drain(..) {
        let id = emote.id;
        check_msg(guild.delete_emoji(&ctx.http, id).await);
    }

    Ok(())
}

#[command("rmdbemotes")]
#[only_in(guilds)]
#[required_permissions("MANAGE_EMOJIS")]
#[num_args(0)]
#[description(
    "Removes all emotes with duplicate names (only one of them) ignoring underscores and case"
)]
async fn delete_duplicate_emotes(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let emotes = guild.emojis(&ctx.http).await?;
    let mut to_delete: HashMap<String, HashSet<&EmojiId>> = HashMap::new();
    for emote in emotes.iter() {
        to_delete
            .entry(str::replace(&emote.name, "_", "").to_lowercase())
            .or_insert(HashSet::new())
            .insert(&emote.id);
    }
    let to_delete = to_delete
        .into_iter()
        .filter(|(_id, dubs)| dubs.len() > 1)
        .map(|(_id, dubs)| dubs)
        .collect::<Vec<HashSet<&EmojiId>>>();
    for mut set in to_delete {
        for id in set.drain().skip(1) {
            check_msg(guild.delete_emoji(&ctx.http, id).await);
        }
    }
    Ok(())
}
