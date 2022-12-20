use std::{collections::HashMap, fmt::Display, sync::Arc};

use crate::{
    error::Error as AYError,
    menu::{self, set_button, Menu},
    util::create_mensa_plan_by_day,
    Context, Error,
};
use chrono::Datelike;
use mensa_fr::{mensa::Plan, mensa::Weekday, MensaPlace, UrlBuilder};
use mensa_swfr_rs as mensa_fr;
use poise::serenity_prelude as serenity;
use strum::IntoEnumIterator;

// TODO: there must be a simpler solution to set the selection of the select menus
// works for now, but not my proudest work

const DEFAULT_PLACE: MensaPlace = MensaPlace::Rempartstraße;

/// shows all mensa plans for freiburg
#[poise::command(slash_command, track_edits, category = "University Freiburg")]
pub async fn mensa(ctx: Context<'_>) -> Result<(), Error> {
    ctx.defer().await?;

    let token = crate::CONFIG.swfr_token.clone().map_or(
        Err(AYError::InvalidInput("no mensa swfr token in config")),
        |token| Ok(token),
    )?;
    let mut mensa_cache = MensaCache::new(&token);
    let weekday_today = chrono::Utc::now().weekday();
    let mensa_today = mensa_cache.mensa_plan(&DEFAULT_PLACE).await?;
    let embed = mensa_today
        .day(weekday_today.into())
        .map(|day| create_mensa_plan_by_day(day))
        .unwrap_or({
            let mut embed = serenity::CreateEmbed::default();
            embed.description("no mensa today").color(crate::color());
            embed
        });

    let weekday_control = menu::Control::new(
        menu::MenuComponent::select("weekday", |button| {
            button.options(|opts| opts.set_options(create_day_options(Some(weekday_today.into()))))
        }),
        Arc::new(|menu, mci| Box::pin(select_weekday(menu, mci))),
    );

    let mensa_control = menu::Control::new(
        menu::MenuComponent::select("mensa", |button| {
            button.options(|opts| opts.set_options(create_mensa_options(Some(DEFAULT_PLACE))))
        }),
        Arc::new(|menu, mci| Box::pin(select_mensa(menu, mci))),
    );

    let mut menu = Menu::new(
        &ctx,
        (Some(chrono::Utc::now().weekday().into()), None, mensa_cache),
        |options| {
            options
                .add_row(|row| row.add_button(weekday_control))
                .add_row(|row| row.add_button(mensa_control))
                .set_timeout(3600)
        },
    );

    menu.run(|m| {
        m.embed(|e| {
            e.clone_from(&embed);
            e
        })
    })
    .await?;
    Ok(())
}

#[derive(Default)]
pub struct MensaCache {
    mensa_token: String,
    cache_map: HashMap<MensaPlace, Plan>,
}

impl<'a> MensaCache {
    pub fn new(mensa_token: &str) -> Self {
        MensaCache {
            mensa_token: mensa_token.to_string(),
            ..Default::default()
        }
    }

    pub async fn mensa_plan(&'a mut self, place: &MensaPlace) -> Result<&'a Plan, Error> {
        let mut url_builder = UrlBuilder::new(&self.mensa_token);
        let future = mensa_fr::request(url_builder.set_place(&place));
        Ok(self.cache_map.entry(*place).or_insert(future.await?))
    }
}

async fn select_weekday(
    menu: &mut Menu<'_, (Option<Weekday>, Option<MensaPlace>, MensaCache)>,
    mci: &Arc<serenity::MessageComponentInteraction>,
) -> Result<(), Error> {
    let dds = &mci.data.values;
    let weekday = dds
        .get(0)
        .map(|day| Weekday::try_from(day.as_str()))
        .unwrap_or(Ok(chrono::Utc::now().weekday().into()))?;
    menu.data.0 = Some(weekday);

    let (day, place, mensa_cache) = &mut menu.data;
    let mut mes = mci.message.clone();
    if let (Some(day), Some(place)) = (&day, &place) {
        let mensa = mensa_cache.mensa_plan(place).await?;
        let embed = mensa
            .day(*day)
            .map(|day| create_mensa_plan_by_day(day))
            .unwrap_or({
                let mut embed = serenity::CreateEmbed::default();
                embed.description("no mensa today").color(crate::color());
                embed
            });
        mes.edit(&menu.ctx.serenity_context(), |edit| {
            edit.set_embed(embed)
                .set_components({
                    let mut comp = serenity::CreateComponents::default();
                    comp.add_action_row({
                        let mut row = serenity::CreateActionRow::default();
                        set_button(
                            &mut row,
                            &menu::MenuComponent::select("weekday", |button| {
                                button.options(|opts| {
                                    opts.set_options(create_day_options(Some(*day)))
                                })
                            }),
                        );
                        row
                    })
                    .add_action_row({
                        let mut row = serenity::CreateActionRow::default();
                        set_button(
                            &mut row,
                            &menu::MenuComponent::select("mensa", |button| {
                                button.options(|opts| {
                                    opts.set_options(create_mensa_options(Some(*place)))
                                })
                            }),
                        );
                        row
                    });

                    comp
                })
                .content("")
        })
        .await?;
    } else if let Some(day) = day {
        mes.edit(&menu.ctx.serenity_context(), |edit| {
            edit.set_components({
                let mut comp = serenity::CreateComponents::default();
                comp.add_action_row({
                    let mut row = serenity::CreateActionRow::default();
                    set_button(
                        &mut row,
                        &menu::MenuComponent::select("weekday", |button| {
                            button.options(|opts| opts.set_options(create_day_options(Some(*day))))
                        }),
                    );
                    row
                })
                .add_action_row({
                    let mut row = serenity::CreateActionRow::default();
                    set_button(
                        &mut row,
                        &menu::MenuComponent::select("mensa", |button| {
                            button.options(|opts| opts.set_options(create_mensa_options(*place)))
                        }),
                    );
                    row
                });

                comp
            })
            .content("Select a Mensa!")
        })
        .await?;
    }
    Ok(())
}

async fn select_mensa(
    menu: &mut Menu<'_, (Option<Weekday>, Option<MensaPlace>, MensaCache)>,
    mci: &Arc<serenity::MessageComponentInteraction>,
) -> Result<(), Error> {
    let dds = &mci.data.values;
    let place = dds
        .get(0)
        .map(|place| MensaPlace::try_from(place.as_str()))
        .unwrap_or(Ok(DEFAULT_PLACE))?;
    menu.data.1 = Some(place);

    let (day, place, mensa_cache) = &mut menu.data;
    if let (Some(day), Some(place)) = (&day, &place) {
        let mensa = mensa_cache.mensa_plan(place).await?;
        let embed = mensa
            .day(*day)
            .map(|day| create_mensa_plan_by_day(day))
            .unwrap_or({
                let mut embed = serenity::CreateEmbed::default();
                embed.description("no mensa today").color(crate::color());
                embed
            });
        let mut mes = mci.message.clone();
        mes.edit(&menu.ctx.serenity_context(), |edit| {
            edit.set_embed(embed)
                .set_components({
                    let mut comp = serenity::CreateComponents::default();
                    comp.add_action_row({
                        let mut row = serenity::CreateActionRow::default();
                        set_button(
                            &mut row,
                            &menu::MenuComponent::select("weekday", |button| {
                                button.options(|opts| {
                                    opts.set_options(create_day_options(Some(*day)))
                                })
                            }),
                        );
                        row
                    })
                    .add_action_row({
                        let mut row = serenity::CreateActionRow::default();
                        set_button(
                            &mut row,
                            &menu::MenuComponent::select("mensa", |button| {
                                button.options(|opts| {
                                    opts.set_options(create_mensa_options(Some(*place)))
                                })
                            }),
                        );
                        row
                    });

                    comp
                })
                .content("")
        })
        .await?;
    }
    Ok(())
}

fn create_options<T: Sized + PartialEq, F: Copy, V: Copy>(
    selected: Option<T>,
    iter: Box<dyn Iterator<Item = T>>,
    display: F,
    value: V,
) -> Vec<serenity::CreateSelectMenuOption>
where
    F: FnOnce(&T) -> Box<dyn Display>,
    V: FnOnce(&T) -> Box<dyn Display>,
{
    let mut options: Vec<serenity::CreateSelectMenuOption> = Vec::new();

    for obj in iter {
        let mut option = serenity::CreateSelectMenuOption::new(display(&obj), value(&obj));
        if Some(obj) == selected {
            option.default_selection(true);
        }
        options.push(option);
    }
    options
}

fn create_mensa_options(selected: Option<MensaPlace>) -> Vec<serenity::CreateSelectMenuOption> {
    create_options(
        selected,
        Box::new(MensaPlace::iter()),
        |place| Box::new(*place),
        |place| Box::new(place.id()),
    )
}

fn create_day_options(selected: Option<Weekday>) -> Vec<serenity::CreateSelectMenuOption> {
    create_options(
        selected,
        Box::new(Weekday::iter()),
        |day| Box::new(day.full_name()),
        |day| Box::new(*day),
    )
}
