use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
struct GuildBase {
    guild_id: u64,
    // TODO: need to reimplement command dispatching
    prefix: String,
    gallery_channel: Option<u64>,
}
