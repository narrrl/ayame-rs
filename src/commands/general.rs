use itertools::Itertools;
use mensa_swfr_rs::mensa::Day;
use std::{collections::HashMap, str::FromStr};

use crate::{
    utils::{create_mensa_plan_by_day, translate_weekday, weekday_german},
    Context, Error,
};
use chrono::{Datelike, Utc, Weekday};
use poise::serenity_prelude::{self as serenity, CreateEmbed, CreateSelectMenuOptions, Invite};

use crate::error::*;

#[poise::command(
    prefix_command,
    slash_command,
    track_edits,
    category = "General",
    ephemeral
)]
pub(crate) async fn mock(
    ctx: Context<'_>,
    #[description = "The text to convert"]
    #[rest]
    text: String,
) -> Result<(), Error> {
    ctx.say(crate::utils::mock_text(&text)).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command, track_edits, category = "General")]
pub(crate) async fn invite(ctx: Context<'_>) -> Result<(), Error> {
    match ctx.guild() {
        Some(_) => {}
        None => {
            return {
                ctx.say("You can only invite into guilds").await?;
                Ok(())
            }
        }
    };
    let inv = Invite::create(&ctx.discord().http, ctx.channel_id(), |f| f).await?;
    ctx.send(|m| m.content(inv.url())).await?;

    Ok(())
}

#[poise::command(prefix_command, slash_command, track_edits, category = "General")]
pub(crate) async fn mensa(
    ctx: Context<'_>,
    #[description = "The day to look up"] day: Option<String>,
) -> Result<(), Error> {
    let config = ctx.data().config.lock().await;
    let mensa_key = config.mensa_api_key();
    drop(&config);
    if let Some(mensa_key) = mensa_key {
        let plan = mensa_swfr_rs::request_rempart(mensa_key).await?;
        let days = plan.days();
        let day = match day {
            Some(day) => match Weekday::from_str(&translate_weekday(&day)) {
                Ok(day) => day,
                Err(_) => return Err(Error::Input(UNKNOWN_WEEKDAY)),
            },
            None => Utc::now().weekday(),
        };

        let uuid = ctx.id();
        let embed = create_mensa_embed(&days, &day);
        ctx.send(|m| {
            m.embed(|e| {
                e.clone_from(&embed);
                e
            })
            .components(|c| {
                c.create_action_row(|ar| {
                    ar.create_select_menu(|menu| {
                        menu.options(|e| create_mensa_options(e, &day, &days))
                            .custom_id(uuid)
                    })
                })
            })
        })
        .await?;

        while let Some(mci) = serenity::CollectComponentInteraction::new(ctx.discord())
            .author_id(ctx.author().id)
            .channel_id(ctx.channel_id())
            .timeout(std::time::Duration::from_secs(120))
            .filter(move |mci| mci.data.custom_id == uuid.to_string())
            .await
        {
            let dds = &mci.data.values;
            if let Some(day) = dds.get(0) {
                let day = match Weekday::from_str(&translate_weekday(&day)) {
                    Ok(day) => day,
                    Err(_) => return Err(Error::Input(UNKNOWN_WEEKDAY)),
                };

                let embed = create_mensa_embed(&days, &day);
                let mut msg = mci.message.clone();
                msg.edit(ctx.discord(), |m| {
                    m.set_embed(embed).components(|c| {
                        c.create_action_row(|ar| {
                            ar.create_select_menu(|menu| {
                                menu.options(|e| create_mensa_options(e, &day, &days))
                                    .custom_id(uuid)
                            })
                        })
                    })
                })
                .await?;

                mci.create_interaction_response(ctx.discord(), |ir| {
                    ir.kind(serenity::InteractionResponseType::DeferredUpdateMessage)
                })
                .await?;
            }
        }
        Ok(())
    } else {
        Err(Error::Failure(NO_MENSA_KEY))
    }
}

fn create_mensa_options<'a>(
    opt: &'a mut CreateSelectMenuOptions,
    day: &Weekday,
    days: &HashMap<Weekday, &Day>,
) -> &'a mut CreateSelectMenuOptions {
    for d in days
        .keys()
        .map(|w| *w)
        .sorted_by(|a, b| Ord::cmp(&(*a as u8), &(*b as u8)))
    {
        opt.create_option(|p| {
            p.label(format!("{}", weekday_german(&d)))
                .value(format!("{}", weekday_german(&d)));
            if d == *day {
                p.default_selection(true);
            }
            p
        });
    }

    opt
}

fn create_mensa_embed(days: &HashMap<Weekday, &Day>, day: &Weekday) -> CreateEmbed {
    match days.get(day) {
        Some(day) => match create_mensa_plan_by_day(day) {
            Ok(embed) => embed,
            Err(_) => {
                let mut e = CreateEmbed::default();
                e.title("Keine Mensa für den ausgewählten Tag");
                return e;
            }
        },
        None => {
            let mut e = CreateEmbed::default();
            e.title("Keine Mensa für den ausgewählten Tag");
            e
        }
    }
}
