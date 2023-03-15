use crate::{error::Error, Context, Data, Result};
use apex_rs::model::Map;
use chrono::{DateTime, Datelike, Utc};
use mensa_swfr_rs::mensa;
use poise::serenity_prelude::{self as serenity, CacheHttp, CreateEmbed};
use regex::Regex;

pub fn type_of<T>() -> &'static str {
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

const SWFR_LOGO: &'static str = "https://cloud.nirusu.codes/s/McBDNYTkNjoEFyc/preview";
pub fn create_mensa_plan_by_day(day: &mensa::Day) -> CreateEmbed {
    let mut embed = CreateEmbed::default();
    embed
        .title(format!(
            "{} ({})",
            &day.weekday()
                .unwrap_or(chrono::Utc::now().weekday().into())
                .full_name(),
            day.to_chrono().unwrap().format("%d.%m.%Y")
        ))
        .color(crate::color())
        .thumbnail(SWFR_LOGO);
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
    embed
}

pub struct Exclusion((u64, u64));

impl Exclusion {
    pub fn users(&self) -> &(u64, u64) {
        &self.0
    }
}

impl From<(i64, i64)> for Exclusion {
    fn from(tuple: (i64, i64)) -> Exclusion {
        Exclusion {
            0: (tuple.0 as u64, tuple.1 as u64),
        }
    }
}
impl From<(u64, u64)> for Exclusion {
    fn from(tuple: (u64, u64)) -> Exclusion {
        Exclusion {
            0: (tuple.0, tuple.1),
        }
    }
}

pub async fn get_user_exclusions(
    database: &sqlx::SqlitePool,
    guild_id: i64,
) -> Result<Vec<Exclusion>> {
    Ok(
        sqlx::query!("SELECT * FROM exclusions WHERE guild_id = ?", guild_id)
            .fetch_all(database)
            .await?
            .iter()
            .map(|record| (record.user_1, record.user_2).into())
            .collect(),
    )
}

pub async fn add_user_exclusion(
    database: &sqlx::SqlitePool,
    guild_id: i64,
    exclusion: &Exclusion,
) -> Result<()> {
    let user_1 = exclusion.0 .0 as i64;
    let user_2 = exclusion.0 .1 as i64;
    sqlx::query!(
        "INSERT INTO exclusions (guild_id, user_1, user_2) VALUES (?, ?, ?)",
        guild_id,
        user_1,
        user_2,
    )
    .execute(database)
    .await?;
    Ok(())
}

pub async fn remove_user_exclusion(
    database: &sqlx::SqlitePool,
    guild_id: i64,
    exclusion: &Exclusion,
) -> Result<()> {
    let user_1 = exclusion.0 .0 as i64;
    let user_2 = exclusion.0 .1 as i64;
    sqlx::query!(
        "DELETE FROM exclusions WHERE guild_id = ? AND user_1 = ? AND user_2 = ?",
        guild_id,
        user_1,
        user_2
    )
    .execute(database)
    .await?;
    Ok(())
}

pub async fn check_for_exclusion_collision(
    ctx: &serenity::Context,
    channel: serenity::Channel,
    data: &Data,
) -> Result<()> {
    let guild_channel = channel
        .guild()
        .map_or(Err(Error::InvalidInput("Not in a guild")), |channel| {
            Ok(channel)
        })?;
    let guild_id = guild_channel.guild_id;
    let exclusions = get_user_exclusions(&data.database, guild_id.into()).await?;
    let users = guild_channel
        .members(
            ctx.cache()
                .map_or(Err(Error::InvalidInput("Cache unavailable")), |cache| {
                    Ok(cache)
                })?,
        )
        .await?
        .iter()
        .map(|member| member.user.id.0)
        .collect::<Vec<u64>>();
    for exclusion in exclusions {
        let (user_1, user_2) = exclusion.users();
        if users.contains(user_1) {
            exclude_user_from_channel(ctx, &guild_channel, *user_2).await?;
        } else if users.contains(user_2) {
            exclude_user_from_channel(ctx, &guild_channel, *user_1).await?;
        }
    }
    Ok(())
}

async fn exclude_user_from_channel(
    ctx: &serenity::Context,
    channel: &serenity::GuildChannel,
    user: u64,
) -> Result<()> {
    channel
        .create_permission(
            ctx.http(),
            &serenity::PermissionOverwrite {
                allow: serenity::Permissions::empty(),
                deny: serenity::Permissions::VIEW_CHANNEL,
                kind: serenity::PermissionOverwriteType::Member(user.into()),
            },
        )
        .await?;
    Ok(())
}
