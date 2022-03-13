use crate::{Context, Error};

#[poise::command(prefix_command, owners_only, hide_in_help)]
pub(crate) async fn shutdown(ctx: Context<'_>) -> Result<(), Error> {
    ctx.framework()
        .shard_manager()
        .lock()
        .await
        .shutdown_all()
        .await;
    Ok(())
}
