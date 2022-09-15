use std::sync::Arc;

use crate::{apex_client, error::Error as AYError, menu, util, Context, Error};
use poise::serenity_prelude as serenity;

/// Show this help menu
#[poise::command(track_edits, slash_command, category = "Apex")]
pub async fn maps(ctx: Context<'_>) -> Result<(), Error> {
    let rotation = apex_client()?.get_map_rotations().await?;
    let rotation = rotation.battle_royal().map_or(
        Err(AYError::Unavailable("map rotations are unavailable")),
        |rot| Ok(rot),
    )?;
    let (current, next) = (rotation.current(), rotation.next());
    let mut menu = menu::Menu::new(&ctx, &rotation, |options| {
        options.add_row(|row| {
            row.add_button(menu::Control::new(
                menu::MenuComponent::button("current", |button| {
                    if current.is_none() {
                        button.disabled(true);
                    }
                    button.style(serenity::ButtonStyle::Primary)
                }),
                Arc::new(|menu, mci| {
                    Box::pin(async {
                        // we can unwrap because the button is disabled
                        // when none
                        let map = menu.data.current().unwrap();
                        menu.update_response(|m| m.set_embed(util::embed_map(map)), mci)
                            .await?;

                        Ok(())
                    })
                }),
            ))
            .add_button(menu::Control::new(
                menu::MenuComponent::button("next", |button| {
                    if next.is_none() {
                        button.disabled(true);
                    }
                    button.style(serenity::ButtonStyle::Primary)
                }),
                Arc::new(|menu, mci| {
                    Box::pin(async {
                        // we can unwrap because the button is disabled
                        // when none
                        let map = menu.data.next().unwrap();
                        menu.update_response(|m| m.set_embed(util::embed_map(map)), mci)
                            .await?;

                        Ok(())
                    })
                }),
            ))
        })
    });

    menu.run(|m| {
        m.embed(|e| match current {
            Some(map) => {
                e.clone_from(&util::embed_map(map));
                e
            }
            None => e.title("Unavailable"),
        })
    })
    .await?;
    Ok(())
}
