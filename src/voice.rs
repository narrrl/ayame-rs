use poise::serenity_prelude as serenity;
use std::{sync::Arc, time::Duration};

struct PlayingHandler {
    guild_id: u64,
    msg_id: Option<u64>,
    sleep_duration: Option<Duration>,
    http: Arc<serenity::Http>,
    songbird: Arc<songbird::Songbird>,
}

impl PlayingHandler {
    pub fn new(
        guild_id: u64,
        http: Arc<serenity::Http>,
        songbird: Arc<songbird::Songbird>,
    ) -> Self {
        PlayingHandler {
            guild_id,
            http,
            songbird,
            msg_id: None,
            sleep_duration: None,
        }
    }

    pub fn set_sleep_duration<'a>(&'a mut self, duration: Duration) -> &'a mut Self {
        self.sleep_duration = Some(duration);
        self
    }

    pub async fn start(handler: &mut PlayingHandler, sleep_int: Duration) {
        let handle = Arc::new(serenity::Mutex::new(handler));
        tokio::spawn(async move {
            let lock = handle.lock().await;
            tokio::time::sleep(lock.sleep_duration.unwrap_or(Duration::from_secs(5))).await;
        });
    }
}
