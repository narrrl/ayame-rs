use crate::{
    util::{add_user_exclusion, get_user_exclusions, remove_user_exclusion, Exclusion},
    Context, Result,
};
use poise::serenity_prelude as serenity;

/// overview about all exclusions on this server
#[poise::command(
    track_edits,
    slash_command,
    subcommands("add", "remove", "list"),
    guild_only
)]
pub async fn exclusions(ctx: Context<'_>) -> Result<()> {
    Ok(())
}

/// list all exclusions on this server
#[poise::command(track_edits, slash_command, guild_only)]
pub async fn list(ctx: Context<'_>) -> Result<()> {
    let exclusions = get_user_exclusions(
        &ctx.data().database,
        ctx.guild_id().unwrap_or(Default::default()).into(),
    )
    .await?;
    ctx.send(|m| m.embed(|embed| embed_exclusions(embed, exclusions)))
        .await?;
    Ok(())
}

pub fn embed_exclusions(
    embed: &mut serenity::CreateEmbed,
    exclusions: Vec<Exclusion>,
) -> &mut serenity::CreateEmbed {
    embed
        .title("All user exclusions on this server")
        .color(crate::color())
        .description(
            "People in the exclusion list can't see each other.
            You can add user that shouldn't see each other with `/exclusions add <user> <user>`
            and remove them with `/exclusions remove <index>`",
        );
    for (index, exclusion) in exclusions.iter().enumerate() {
        let (id_1, id_2) = exclusion.users();
        embed.field(
            format!("{} Exclusion", index),
            format!("<@{}> ignores <@{}>", id_1, id_2),
            false,
        );
    }
    embed
}

/// add two user that should ignore each other
#[poise::command(track_edits, slash_command, guild_only)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "first user that ignores the second"] user_1: serenity::User,
    #[description = "second user that ignores the first"] user_2: serenity::User,
) -> Result<()> {
    let guild_id: u64 = ctx.guild_id().unwrap_or_default().into();
    let exclusion = Exclusion::from((user_1.id.0, user_2.id.0));
    add_user_exclusion(&ctx.data().database, guild_id as i64, &exclusion).await?;
    ctx.say("Added to the exclusion list").await?;
    Ok(())
}

/// remove a user exclusion
#[poise::command(track_edits, slash_command, guild_only)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "first user that ignores the second"] user_1: serenity::User,
    #[description = "second user that ignores the first"] user_2: serenity::User,
) -> Result<()> {
    let guild_id: u64 = ctx.guild_id().unwrap_or_default().into();
    let exclusion = Exclusion::from((user_1.id.0, user_2.id.0));
    remove_user_exclusion(&ctx.data().database, guild_id as i64, &exclusion).await?;
    ctx.say("Removed from the exclusion list").await?;
    Ok(())
}
