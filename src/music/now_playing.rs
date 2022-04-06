use poise::serenity_prelude as serenity;

use crate::error::{AyameError as Error, NOT_IN_VOICE};

use super::get_poise;

pub async fn now_playing(
    ctx: &crate::Context<'_>,
    guild_id: &serenity::GuildId,
) -> Result<(), crate::Error> {
    let songbird = &get_poise(ctx).await?;

    let call = match songbird.get(*guild_id.as_u64()) {
        Some(call) => call,
        None => return Err(Error::Input(NOT_IN_VOICE)),
    };
    let call = call.lock().await;

    let queue = call.queue();
    let current = queue.current();
    let embed =
        super::get_now_playing_embed(&ctx.data(), &current, queue, &ctx.discord().cache).await;
    let reply = ctx
        .send(|m| {
            m.embed(|e| {
                e.clone_from(&embed);
                e
            })
        })
        .await?;
    let mut ids = ctx.data().playing_messages.lock().await;

    ids.insert(*guild_id, reply.message().await?.id);
    Ok(())
}
