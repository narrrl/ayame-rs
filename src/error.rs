use poise::async_trait;
use std::error::Error as StdError;
use std::fmt;
use tracing::instrument;

use crate::Context;

#[async_trait]
pub trait Sendable<T, E>
where
    T: Send + Sync,
    E: Send + Sync,
{
    async fn send(&self, ctx: &T) -> std::result::Result<E, Box<dyn StdError + Send + Sync>>;
}

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    InvalidInput(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(msg) => f.write_str(msg),
        }
    }
}

impl StdError for Error {
    #[instrument]
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            _ => None,
        }
    }
}

#[async_trait]
impl Sendable<Context<'_>, ()> for Error {
    async fn send(
        &self,
        ctx: &Context,
    ) -> std::result::Result<(), Box<dyn StdError + Send + Sync>> {
        ctx.send(|m| {
            m.embed(|e| {
                e.description(format!("Error: {}", &self))
                    .colour(*crate::COLOR)
            })
            .ephemeral(true)
        })
        .await?;
        Ok(())
    }
}
