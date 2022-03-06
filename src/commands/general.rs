use crate::{Context, Error};
use poise::serenity_prelude as serenity;

#[poise::command(prefix_command, slash_command, track_edits, category = "General")]
pub(crate) async fn mock(
    ctx: Context<'_>,
    #[description = "The text to convert"] text: String,
) -> Result<(), Error> {
    ctx.say(crate::utils::mock_text(&text)).await?;
    Ok(())
}

pub(crate) async fn mensa(ctx: Context<'_>, day: Option<String>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(prefix_command, slash_command, category = "General")]
pub(crate) async fn boop(ctx: Context<'_>) -> Result<(), Error> {
    let uuid_boop = ctx.id();

    ctx.send(|m| {
        m.content("I want some boops!").components(|c| {
            c.create_action_row(|ar| {
                ar.create_button(|b| {
                    b.style(serenity::ButtonStyle::Primary)
                        .label("Boop me!")
                        .custom_id(uuid_boop)
                })
            })
        })
    })
    .await?;

    let mut boop_count = 0;
    while let Some(mci) = serenity::CollectComponentInteraction::new(ctx.discord())
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(std::time::Duration::from_secs(120))
        .filter(move |mci| mci.data.custom_id == uuid_boop.to_string())
        .await
    {
        boop_count += 1;

        let mut msg = mci.message.clone();
        msg.edit(ctx.discord(), |m| {
            m.content(format!("Boop count: {}", boop_count))
        })
        .await?;

        mci.create_interaction_response(ctx.discord(), |ir| {
            ir.kind(serenity::InteractionResponseType::DeferredUpdateMessage)
        })
        .await?;
    }

    Ok(())
}
