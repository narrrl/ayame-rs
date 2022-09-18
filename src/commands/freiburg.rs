use std::collections::HashMap;

use crate::{
    error::Error as AYError,
    menu::{self, Menu},
    Context, Error,
};
use mensa_fr::{mensa::Plan, MensaPlace, UrlBuilder};
use mensa_swfr_rs as mensa_fr;
use poise::serenity_prelude as serenity;

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

    let mut menu = Menu::new(&ctx, mensa_cache, |options| options);

    // menu.run(|m| m).await?;
    Err(Box::new(AYError::Unavailable("not implemented")))
}

#[derive(Default)]
pub struct MensaCache<'a> {
    mensa_token: &'a str,
    cache_map: HashMap<MensaPlace, Plan>,
}

impl<'a> MensaCache<'a> {
    pub fn new(mensa_token: &'a str) -> Self {
        MensaCache {
            mensa_token,
            ..Default::default()
        }
    }

    pub async fn mensa_plan(&'a mut self, place: MensaPlace) -> Result<&'a Plan, Error> {
        let mut url_builder = UrlBuilder::new(self.mensa_token);
        let future = mensa_fr::request(url_builder.set_place(&place));
        Ok(self.cache_map.entry(place).or_insert(future.await?))
    }
}
