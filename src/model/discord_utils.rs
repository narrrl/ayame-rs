use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use serenity::{
    builder::{CreateEmbed, CreateMessage},
    client::Context,
    collector::ReactionAction,
    futures::StreamExt,
    model::{
        channel::{Message, Reaction, ReactionType},
        guild::{Guild, PremiumTier},
        id::{ChannelId, UserId},
    },
    utils::Color,
    Result as SerenityResult,
};

use tracing::error;

use crate::error::Error;

pub type Result<T> = std::result::Result<T, Error>;

pub fn default_embed() -> CreateEmbed {
    let mut e = CreateEmbed::default();
    set_defaults_for_embed(&mut e);
    e
}

pub fn set_defaults_for_embed(e: &mut CreateEmbed) {
    e.color(Color::from_rgb(238, 14, 97));
    let time = chrono::Utc::now();
    e.timestamp(&time);
    // add more defaults for embeds ...
}

pub fn set_defaults_for_error(e: &mut CreateEmbed, message: &str) {
    // set defaults
    set_defaults_for_embed(e);
    // red color to indicate error
    e.color(Color::from_rgb(204, 0, 0));
    e.title(format!("Error: {}", message));
}

pub fn get_max_uploadsize(guild: &Guild) -> u64 {
    let tier = guild.premium_tier;
    match tier {
        PremiumTier::Tier2 => 50_000_000,
        PremiumTier::Tier3 => 100_000_000,
        _ => 8_000_000,
    }
}

pub fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        error!("Error sending message: {:?}", why);
    }
}

// heavily inspired by
// [serenity_utils](https://github.com/AriusX7/serenity-utils/blob/current/src/menu.rs)
// modified for my needs, including slash command support

pub struct SelectMenu<'a> {
    ctx: &'a Context,
    author_id: &'a UserId,
    channel_id: &'a ChannelId,
    pages: &'a Vec<CreateMessage<'a>>,
    options: MusicSelectOptions,
    has_selected: bool,
    was_canceled: bool,
}

impl<'a> SelectMenu<'a> {
    pub fn new(
        ctx: &'a Context,
        author_id: &'a UserId,
        channel_id: &'a ChannelId,
        pages: &'a Vec<CreateMessage<'a>>,
        options: MusicSelectOptions,
    ) -> Self {
        Self {
            ctx,
            author_id,
            channel_id,
            pages,
            options,
            has_selected: false,
            was_canceled: false,
        }
    }

    pub async fn run(mut self) -> Result<(usize, Option<Message>)> {
        if let Some(msg) = &self.options.message {
            self.add_reactions(&msg).await?;
        }

        loop {
            if self.has_selected {
                break;
            } else if self.was_canceled {
                return Err(Error::from("selection was canceled by user"));
            }
            match self.work().await {
                Ok((index, reaction)) => match self.options.controls.get(index) {
                    Some(control) => {
                        Arc::clone(&control.function)(&mut self, reaction).await;
                    }
                    None => {
                        let _ = self.clean_reactions().await;
                        break;
                    }
                },
                Err(e) => {
                    let _ = self.clean_reactions().await;

                    return Err(e);
                }
            }
        }

        Ok((self.options.current_page, self.options.message))
    }

    async fn work(&mut self) -> Result<(usize, Reaction)> {
        if self.pages.is_empty() {
            return Err(Error::from("`pages` is empty."));
        }

        if self.options.current_page > self.pages.len() - 1 {
            return Err(Error::from("`page` is out of bounds."));
        }
        let page = &self.pages.get(self.options.current_page).unwrap();
        match &mut self.options.message {
            Some(m) => {
                m.edit(&self.ctx.http, |m| {
                    m.0.clone_from(&page.0);
                    m
                })
                .await?;
            }
            None => {
                let msg = self
                    .channel_id
                    .send_message(&self.ctx.http, |m| {
                        m.clone_from(page);
                        m
                    })
                    .await?;
                self.add_reactions(&msg).await?;
                self.options.message = Some(msg);
            }
        };
        let message = self.options.message.as_ref().unwrap();
        let mut reaction_collector = message
            .await_reactions(&self.ctx)
            .timeout(Duration::from_secs(self.options.timeout))
            .author_id(self.author_id.as_u64().clone())
            .await;

        let (choice, reaction) = {
            let mut choice = None;
            let mut reaction = None;

            let mut found_one = false;

            while let Some(item) = reaction_collector.next().await {
                if let ReactionAction::Added(r) = item.as_ref() {
                    if !found_one {
                        found_one = true;
                    }

                    let r = r.as_ref().clone();
                    if let Some(i) = self.process_reaction(&r) {
                        choice = Some(i);
                        reaction = Some(r);
                        break;
                    }
                }
            }

            if !found_one {
                return Err(Error::TimeoutError);
            }

            (choice, reaction)
        };
        match choice {
            Some(c) => Ok((c, reaction.unwrap())),
            None => Err(Error::InvalidInput),
        }
    }

    async fn add_reactions(&self, msg: &Message) -> Result<()> {
        let emojis = self
            .options
            .controls
            .iter()
            .map(|c| c.emoji.clone())
            .collect::<Vec<_>>();

        add_reactions(self.ctx, msg, emojis).await?;

        Ok(())
    }
    fn process_reaction(&self, reaction: &Reaction) -> Option<usize> {
        let emoji = &reaction.emoji;

        for (idx, control) in self.options.controls.iter().enumerate() {
            if &control.emoji == emoji {
                return Some(idx);
            }
        }

        None
    }

    async fn clean_reactions(&self) -> Result<()> {
        if let Some(msg) = &self.options.message {
            msg.delete_reactions(&self.ctx.http).await?;
        }

        Ok(())
    }
}

pub struct MusicSelectOptions {
    current_page: usize,
    timeout: u64,
    message: Option<Message>,
    controls: Vec<Control>,
}

impl MusicSelectOptions {
    pub fn new(timeout: u64, controls: Vec<Control>) -> Self {
        Self {
            current_page: 0,
            timeout,
            message: None,
            controls,
        }
    }

    pub fn set_message(&mut self, message: Message) -> &mut Self {
        self.message = Some(message);
        self
    }
}

impl Default for MusicSelectOptions {
    fn default() -> Self {
        let controls = vec![
            Control::new('◀'.into(), Arc::new(|m, r| Box::pin(prev_page(m, r)))),
            Control::new('✅'.into(), Arc::new(|m, r| Box::pin(select_page(m, r)))),
            Control::new('🚫'.into(), Arc::new(|m, r| Box::pin(cancel(m, r)))),
            Control::new('▶'.into(), Arc::new(|m, r| Box::pin(next_page(m, r)))),
        ];
        MusicSelectOptions::new(30, controls)
    }
}

pub struct Control {
    emoji: ReactionType,
    function: ControlFunction,
}

impl Control {
    pub fn new(emoji: ReactionType, function: ControlFunction) -> Self {
        Self { emoji, function }
    }
}

pub type ControlFunction = Arc<
    dyn for<'b> Fn(
            &'b mut SelectMenu<'_>,
            Reaction,
        ) -> Pin<Box<dyn Future<Output = ()> + 'b + Send>>
        + Sync
        + Send,
>;

pub async fn next_page(menu: &mut SelectMenu<'_>, reaction: Reaction) {
    let _ = reaction.delete(&menu.ctx.http).await;

    if menu.options.current_page == menu.pages.len() - 1 {
        menu.options.current_page = 0;
    } else {
        menu.options.current_page += 1;
    }
}

pub async fn prev_page(menu: &mut SelectMenu<'_>, reaction: Reaction) {
    let _ = reaction.delete(&menu.ctx.http).await;

    if menu.options.current_page == 0 {
        menu.options.current_page = menu.pages.len() - 1;
    } else {
        menu.options.current_page -= 1;
    }
}

pub async fn select_page(menu: &mut SelectMenu<'_>, _reaction: Reaction) {
    menu.has_selected = true;
}

pub async fn cancel(menu: &mut SelectMenu<'_>, _reaction: Reaction) {
    menu.was_canceled = true;
}

pub async fn add_reactions(ctx: &Context, msg: &Message, emojis: Vec<ReactionType>) -> Result<()> {
    let channel_id = msg.channel_id;
    let msg_id = msg.id;
    let http = ctx.http.clone();

    tokio::spawn(async move {
        for emoji in emojis {
            http.create_reaction(channel_id.0, msg_id.0, &emoji).await?;
        }

        Result::<_>::Ok(())
    });

    Ok(())
}
