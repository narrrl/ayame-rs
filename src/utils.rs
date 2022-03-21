use chrono::Weekday;
use core::fmt;
use mensa_swfr_rs::mensa;
use regex::Regex;
use std::{fmt::Display, future::Future, path::PathBuf, pin::Pin, sync::Arc};

use poise::{
    serenity::Result as SerenityResult,
    serenity_prelude::{
        self as serenity, CreateActionRow, CreateButton, CreateEmbed, CreateInputText,
        CreateSelectMenu, MessageId,
    },
};
use rand::Rng;

use tracing::error;

use crate::{error::*, Context, Error};

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
    pages: &'a Vec<CreateEmbed>,
    options: SelectMenuOptions,
    has_selected: bool,
    was_canceled: bool,
}

impl<'a> SelectMenu<'a> {
    pub fn new(
        ctx: &'a Context<'a>,
        pages: &'a Vec<CreateEmbed>,
        options: SelectMenuOptions,
    ) -> Result<Self, Error> {
        if pages.is_empty() {
            return Err(Error::Input(EMPTY_MENU));
        }
        Ok(Self {
            ctx,
            pages,
            options,
            has_selected: false,
            was_canceled: false,
        })
    }

    pub async fn run(mut self) -> Result<(usize, Option<MessageId>), Error> {
        self.register().await?;

        while let Some(mci) = serenity::CollectComponentInteraction::new(self.ctx.discord())
            .author_id(self.ctx.author().id)
            .channel_id(self.ctx.channel_id())
            .timeout(std::time::Duration::from_secs(self.options.timeout))
            .await
        {
            let action = Arc::clone(
                &self
                    .options
                    .controls
                    .iter()
                    .flatten()
                    .find(|ctrl| ctrl.button.id() == mci.data.custom_id)
                    .ok_or_else(|| Error::Failure(UNKNOWN_RESPONSE))?
                    .function,
            );
            action(&mut self).await;
            mci.create_interaction_response(self.ctx.discord(), |ir| {
                ir.kind(serenity::InteractionResponseType::DeferredUpdateMessage)
            })
            .await?;
            if self.was_canceled {
                return Err(Error::Input(EVENT_CANCELED));
            }
            if self.has_selected {
                break;
            }
            if let Some(msg_id) = self.options.msg_id {
                let page = self
                    .pages
                    .get(self.options.current_page)
                    .ok_or_else(|| Error::Failure(UNKNOWN_RESPONSE))?;
                self.ctx
                    .discord()
                    .http
                    .get_message(self.ctx.channel_id().into(), msg_id.into())
                    .await?
                    .edit(&self.ctx.discord().http, |m| m.set_embed(page.clone()))
                    .await?;
            }
        }
        if let Some(msg_id) = self.options.msg_id {
            if self.options.delete_msg {
                self.ctx
                    .discord()
                    .http
                    .delete_message(self.ctx.channel_id().into(), msg_id.into())
                    .await?;
            }
        }
        Ok((self.options.current_page, self.options.msg_id))
    }

    async fn register(&mut self) -> Result<(), Error> {
        let msg_handle = self
            .ctx
            .send(|m| {
                if let Some(page) = self.pages.get(self.options.current_page) {
                    m.embed(|e| {
                        e.clone_from(page);
                        e
                    });
                }
                m.components(|c| {
                    for row in self.options.controls.iter() {
                        c.create_action_row(|a| create_action_row(a, row));
                    }
                    c
                })
            })
            .await?;
        let msg_handle = msg_handle.ok_or_else(|| Error::Failure(COULDNT_GET_MSG))?;
        self.options.msg_id = Some(msg_handle.message().await?.id);
        Ok(())
    }
}

pub struct SelectMenuOptions {
    current_page: usize,
    timeout: u64,
    msg_id: Option<MessageId>,
    controls: Vec<Vec<Control>>,
    delete_msg: bool,
}

impl SelectMenuOptions {
    pub fn new(
        current_page: usize,
        timeout: u64,
        msg_id: Option<MessageId>,
        controls: Vec<Vec<Control>>,
        delete_msg: bool,
    ) -> Self {
        Self {
            current_page,
            timeout,
            msg_id,
            controls,
            delete_msg,
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
            delete_msg: false,
        }
    }
}

pub struct Control {
    button: MenuComponent,
    function: ControlFunction,
}

impl Control {
    pub fn new(button: MenuComponent, function: ControlFunction) -> Self {
        Self { button, function }
    }
}

pub type ControlFunction = Arc<
    dyn for<'b> Fn(&'b mut SelectMenu<'_>) -> Pin<Box<dyn Future<Output = ()> + 'b + Send>>
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

impl MenuComponent {
    fn id(&self) -> String {
        match self {
            Self::ButtonComponent { create: _, id } => id,
            Self::SelectComponent { create: _, id } => id,
            Self::InputComponent { create: _, id } => id,
        }
        .clone()
    }

    pub fn button<F>(id: &str, f: F) -> MenuComponent
    where
        F: FnOnce(&mut CreateButton) -> &mut CreateButton,
    {
        let mut b = CreateButton::default();
        Self::ButtonComponent {
            create: f(&mut b).clone(),
            id: id.to_string(),
        }
    }
    #[allow(dead_code)]
    pub fn select<F>(id: &str, f: F) -> MenuComponent
    where
        F: FnOnce(&mut CreateSelectMenu) -> &mut CreateSelectMenu,
    {
        let mut b = CreateSelectMenu::default();
        Self::SelectComponent {
            create: f(&mut b).clone(),
            id: id.to_string(),
        }
    }
    #[allow(dead_code)]
    pub fn input<F>(id: &str, f: F) -> MenuComponent
    where
        F: FnOnce(&mut CreateInputText) -> &mut CreateInputText,
    {
        let mut b = CreateInputText::default();
        Self::InputComponent {
            create: f(&mut b).clone(),
            id: id.to_string(),
        }
    }
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

// some default functions for menus
pub async fn next_page(menu: &mut SelectMenu<'_>) {
    if menu.options.current_page == menu.pages.len() - 1 {
        menu.options.current_page = 0;
    } else {
        menu.options.current_page += 1;
    }
}

pub async fn prev_page(menu: &mut SelectMenu<'_>) {
    if menu.options.current_page == 0 {
        menu.options.current_page = menu.pages.len() - 1;
    } else {
        menu.options.current_page -= 1;
    }
}

pub async fn select_page(menu: &mut SelectMenu<'_>) {
    menu.has_selected = true;
}

pub async fn cancel(menu: &mut SelectMenu<'_>) {
    menu.was_canceled = true;
}
