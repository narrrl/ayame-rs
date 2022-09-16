use std::sync::Arc;

use crate::{
    apex_client,
    error::Error as AYError,
    menu::{self, Menu},
    util, Context, Error,
};
use apex_rs::model::Map;
use poise::serenity_prelude as serenity;

/// get the current map rotation for battle royal in apex legends
#[poise::command(track_edits, slash_command, category = "Apex")]
pub async fn maps(ctx: Context<'_>) -> Result<(), Error> {
    // defer to let discord now that the command worked
    ctx.defer().await?;
    // the map rotations
    let rotation = apex_client()?.battle_royal_rotation().await?;

    // get the current and next map for battle royal
    // return error if a map is unavailable
    // invalid input to inform user that the apex api is not reachable
    let (current, next) = (
        rotation.current().map_or(
            Err(AYError::InvalidInput("maps are currently not available")),
            |map| Ok(map),
        )?,
        rotation.next().map_or(
            Err(AYError::InvalidInput("maps are currently not available")),
            |map| Ok(map),
        )?,
    );

    // button and control function for the current map
    let button_current = menu::MenuComponent::button("current", |button| {
        button
            .style(serenity::ButtonStyle::Primary)
            .label("current")
    });
    let action_current = menu::Control::new(
        button_current,
        Arc::new(|menu, mci| Box::pin(select_current(menu, mci))),
    );

    // button and control function for the next map
    let next_button = menu::MenuComponent::button("next", |button| {
        button.style(serenity::ButtonStyle::Primary).label("next")
    });
    let next_action = menu::Control::new(
        next_button,
        Arc::new(|menu, mci| Box::pin(select_next(menu, mci))),
    );

    // create the menu with the buttons
    let mut menu = menu::Menu::new(&ctx, (current, next), |options| {
        options.add_row(|row| row.add_button(action_current).add_button(next_action))
    });

    // run the menu with the current map as default
    menu.run(|m| {
        m.embed(|e| {
            e.clone_from(&util::embed_map(current, false));
            e
        })
    })
    .await
}

/// the function to select the current map as display
async fn select_current(
    m: &mut Menu<'_, (&Map, &Map)>,
    mci: &Arc<serenity::MessageComponentInteraction>,
) -> Result<(), Error> {
    let map = m.data.0;
    m.update_response(|m| m.set_embed(util::embed_map(map, false)), mci)
        .await?;

    Ok(())
}

/// the function to select the next map as display
async fn select_next(
    m: &mut Menu<'_, (&Map, &Map)>,
    mci: &Arc<serenity::MessageComponentInteraction>,
) -> Result<(), Error> {
    let map = m.data.1;
    m.update_response(|m| m.set_embed(util::embed_map(map, true)), mci)
        .await?;

    Ok(())
}
