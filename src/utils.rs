use chrono::Weekday;
use core::fmt;
use mensa_swfr_rs::mensa;
use regex::Regex;
use std::{fmt::Display, future::Future, path::PathBuf, pin::Pin, sync::Arc};

use poise::{
    serenity::Result as SerenityResult,
    serenity_prelude::{
        self as serenity, Button, ChannelId, CreateActionRow, CreateButton, CreateEmbed,
        CreateInputText, CreateMessage, CreateSelectMenu, Message, MessageId, UserId,
    },
};
use rand::Rng;

use tracing::error;

use crate::{Context, Error};

pub const NOT_IN_GUILD: &'static str = "only in guilds";

pub fn bot_dir() -> PathBuf {
    let mut dir = std::env::current_exe().expect("couldn't get bot directory");
    dir.pop();
    dir
}

pub fn get_file(name: &str) -> PathBuf {
    let mut dir = bot_dir();
    dir.push(name);
    dir
}

pub fn mock_text(text: &str) -> String {
    let mut mock_str = String::new();

    let mut rng = rand::thread_rng();

    for ch in text.chars() {
        if rng.gen() {
            mock_str.push_str(&ch.to_uppercase().collect::<String>());
        } else {
            mock_str.push_str(&ch.to_lowercase().collect::<String>());
        }
    }
    mock_str
}

pub fn create_mensa_plan_by_day(
    day: &mensa::Day,
) -> Result<CreateEmbed, Box<dyn std::error::Error>> {
    let mut embed = CreateEmbed::default();
    embed.title(format!(
        "{} ({})",
        weekday_german(&day.weekday()?),
        day.to_chrono()?.format("%d.%m.%Y")
    ));
    for menu in day.menues.iter() {
        let price = &menu.price;
        embed.field(
            &menu.art,
            format!(
                "{}\n\nZusatz: {}\n\nPreis: {}/{}/{}",
                Regex::new(r"--+").unwrap().replace(&menu.name, "\n\n!!! "),
                match &menu.food_type {
                    Some(typ) => typ,
                    None => "None",
                },
                price.price_students,
                price.price_workers,
                price.price_guests,
            ),
            false,
        );
    }
    Ok(embed)
}

pub fn translate_weekday(wd: &str) -> String {
    match wd.to_lowercase().as_str() {
        "montag" | "mo" => format!("{}", Weekday::Mon),
        "dienstag" | "di" => format!("{}", Weekday::Tue),
        "mittwoch" | "mi" => format!("{}", Weekday::Wed),
        "donnerstag" | "do" => format!("{}", Weekday::Thu),
        "freitag" | "fr" => format!("{}", Weekday::Fri),
        "samstag" | "sa" => format!("{}", Weekday::Sat),
        "sonntag" | "so" => format!("{}", Weekday::Sun),
        _ => String::from(wd),
    }
}

pub fn weekday_german(wd: &Weekday) -> String {
    match wd {
        Weekday::Mon => String::from("Montag"),
        Weekday::Tue => String::from("Dienstag"),
        Weekday::Wed => String::from("Mittwoch"),
        Weekday::Thu => String::from("Donnerstag"),
        Weekday::Fri => String::from("Freitag"),
        Weekday::Sat => String::from("Samstag"),
        Weekday::Sun => String::from("Sonntag"),
    }
}

pub fn check_result<T>(result: SerenityResult<T>) {
    if let Err(why) = result {
        error!("error: {:?}", why);
    }
}

pub fn check_result_ayame<T>(result: Result<T, Error>) {
    if let Err(why) = result {
        error!("error: {:?}", why);
    }
}

pub(crate) async fn guild_only(ctx: Context<'_>) -> Result<bool, Error> {
    match ctx.guild() {
        Some(_) => Ok(true),
        None => Err(Error::Input(NOT_IN_GUILD)),
    }
}

pub struct Bar {
    pub pos_icon: String,

    pub line_icon: String,

    pub length: usize,

    pub pos: f64,
}

impl Default for Bar {
    fn default() -> Bar {
        Bar {
            pos_icon: ">".to_string(),
            line_icon: "=".to_string(),
            length: 30,
            pos: 0.0f64,
        }
    }
}

impl Bar {
    pub fn set<'a>(&'a mut self, pos: f64) -> &'a mut Bar {
        self.pos = pos;
        self
    }

    pub fn set_len<'a>(&'a mut self, len: usize) -> &'a mut Bar {
        self.length = len;
        self
    }
}

impl Display for Bar {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        let r: usize = (self.pos * self.length as f64) as usize;
        let prev = self.line_icon.repeat(r);
        let (h, next) = if r >= self.length {
            (String::new(), String::new())
        } else {
            (
                String::from(&self.pos_icon),
                self.line_icon.repeat(self.length - r - 1),
            )
        };
        write!(fmt, "[{}{}{}]", prev, h, next)
    }
}

pub struct SelectMenu<'a> {
    ctx: &'a Context<'a>,
    pages: &'a Vec<CreateMessage<'a>>,
    options: SelectMenuOptions,
    has_selected: bool,
    was_canceled: bool,
}

impl<'a> SelectMenu<'a> {
    pub fn new(
        ctx: &'a Context<'a>,
        pages: &'a Vec<CreateMessage<'a>>,
        options: SelectMenuOptions,
    ) -> Self {
        Self {
            ctx,
            pages,
            options,
            has_selected: false,
            was_canceled: false,
        }
    }

    pub async fn run(mut self) -> Result<(usize, Option<Message>), Error> {
        self.register().await?;

        while let Some(mci) = serenity::CollectComponentInteraction::new(self.ctx.discord())
            .author_id(self.ctx.author().id)
            .channel_id(self.ctx.channel_id())
            .timeout(std::time::Duration::from_secs(self.options.timeout))
            .await
        {}

        // TODO: implement
        unimplemented!();
    }

    async fn register(&mut self) -> Result<(), Error> {
        self.ctx
            .send(|m| {
                m.components(|c| {
                    for row in self.options.controls.iter() {
                        c.create_action_row(|a| create_action_row(a, row));
                    }
                    c
                })
            })
            .await?;
        Ok(())
    }
}

pub struct SelectMenuOptions {
    current_page: usize,
    timeout: u64,
    msg_id: Option<MessageId>,
    controls: Vec<Vec<Control>>,
}

impl SelectMenuOptions {
    pub fn new(
        current_page: usize,
        timeout: u64,
        msg_id: Option<MessageId>,
        controls: Vec<Control>,
    ) -> Self {
        Self {
            current_page,
            timeout,
            msg_id,
            controls,
        }
    }
}

impl Default for SelectMenuOptions {
    fn default() -> SelectMenuOptions {
        SelectMenuOptions {
            current_page: 0,
            timeout: 120,
            msg_id: None,
            controls: vec![],
        }
    }
}

pub struct Control {
    button: MenuComponent,
    function: ControlFunction,
}

pub type ControlFunction = Arc<
    dyn for<'b> Fn(&'b mut SelectMenu<'_>, Button) -> Pin<Box<dyn Future<Output = ()> + 'b + Send>>
        + Sync
        + Send,
>;

pub enum MenuComponent {
    ButtonComponent {
        create: CreateButton,
        id: String,
    },
    InputComponent {
        create: CreateInputText,
        id: String,
    },
    SelectComponent {
        create: CreateSelectMenu,
        id: String,
    },
}

fn create_action_row<'b>(
    a: &'b mut CreateActionRow,
    buttons: &Vec<Control>,
) -> &'b mut CreateActionRow {
    for ctrl in buttons {
        match &ctrl.button {
            MenuComponent::ButtonComponent { create, id } => a.create_button(|b| {
                b.clone_from(&create);
                b.custom_id(id)
            }),
            MenuComponent::InputComponent { create, id } => a.create_input_text(|i| {
                i.clone_from(&create);
                i.custom_id(id)
            }),
            MenuComponent::SelectComponent { create, id } => a.create_select_menu(|sm| {
                sm.clone_from(&create);
                sm.custom_id(id)
            }),
        };
    }
    a
}
