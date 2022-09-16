use std::{future::Future, pin::Pin, sync::Arc};

use crate::{error::Error::InvalidInput, Context, Error};
use poise::{serenity_prelude as serenity, CreateReply};

pub struct Menu<'a, T> {
    pub ctx: &'a Context<'a>,
    options: MenuOptions<T>,
    pub data: T,
    is_runnig: bool,
}

impl<'a, T> Menu<'a, T> {
    pub fn new(
        ctx: &'a Context<'a>,
        data: T,
        f: impl for<'b> FnOnce(&'b mut CreateMenuOptions<T>) -> &'b mut CreateMenuOptions<T>,
    ) -> Self {
        let mut co = CreateMenuOptions::default();
        f(&mut co);
        Self {
            ctx,
            data,
            options: co.build(),
            is_runnig: true,
        }
    }

    pub async fn run(
        &mut self,
        f: impl for<'b, 'c> FnOnce(&'b mut CreateReply<'c>) -> &'b mut CreateReply<'c>,
    ) -> Result<(), Error> {
        // TODO: this fails for some reason
        let mes = &self.send_msg(f).await?;
        if let Some(pre_hook) = &self.options.pre_hook {
            Arc::clone(pre_hook)(self).await?;
        }
        while let Some(mci) = serenity::CollectComponentInteraction::new(self.ctx.discord())
            .author_id(self.ctx.author().id)
            .channel_id(self.ctx.channel_id())
            .timeout(std::time::Duration::from_secs(self.options.timeout))
            .await
        {
            let m = &mci.message;
            if let Err(why) = self.match_and_run(&mci).await {
                if let Some(post_hook) = &self.options.post_hook {
                    Arc::clone(post_hook)(self).await?;
                }
                return Err(why);
            }
            // respond and ignore error if already responded
            //
            let _ = mci.defer(&self.ctx.discord().http).await;

            if !self.is_runnig {
                let _ = m.delete(&self.ctx.discord().http).await;
                break;
            }
        }
        let _ = mes.delete(&self.ctx.discord().http).await;

        if let Some(post_hook) = &self.options.post_hook {
            Arc::clone(post_hook)(self).await?;
        }
        Ok(())
    }

    async fn match_and_run(
        &mut self,
        mci: &Arc<serenity::MessageComponentInteraction>,
    ) -> Result<(), Error> {
        let action = &self
            .options
            .controls
            .iter()
            .map(|row| &row.buttons)
            .flatten()
            .find(|ctrl| ctrl.button.id() == mci.data.custom_id)
            .ok_or_else(|| InvalidInput("got unknown interaction"))?;
        let func = Arc::clone(&action.function);

        // run function of button/context
        func(self, &mci).await?;
        Ok(())
    }
    pub async fn send_msg(
        &self,
        f: impl for<'b, 'c> FnOnce(&'b mut CreateReply<'c>) -> &'b mut CreateReply<'c>,
    ) -> Result<serenity::Message, Error> {
        let handle = self
            .ctx
            .send(|m| {
                f(m).components(|cs| {
                    for row in self.options.controls.iter() {
                        cs.add_action_row(row.action_row());
                    }
                    cs
                })
            })
            .await?;
        Ok(handle.message().await?.into_owned())
    }

    pub async fn update_response(
        &self,
        f: impl for<'b, 'c> FnOnce(
            &'b mut serenity::CreateInteractionResponseData<'c>,
        ) -> &'b mut serenity::CreateInteractionResponseData<'c>,
        mci: &Arc<serenity::MessageComponentInteraction>,
    ) -> Result<(), Error> {
        mci.create_interaction_response(&self.ctx.discord(), |ir| {
            ir.kind(serenity::InteractionResponseType::UpdateMessage)
                .interaction_response_data(|m| f(m))
        })
        .await?;
        Ok(())
    }

    pub fn stop(&mut self) {
        self.is_runnig = false;
    }
}

pub struct CreateMenuOptions<T> {
    timeout: u64,
    controls: Vec<ControlRow<T>>,
    pre_hook: Option<HookFunction<T>>,
    post_hook: Option<HookFunction<T>>,
}

impl<T> Default for CreateMenuOptions<T> {
    fn default() -> CreateMenuOptions<T> {
        CreateMenuOptions {
            timeout: 120,
            controls: vec![],
            pre_hook: None,
            post_hook: None,
        }
    }
}

impl<T> CreateMenuOptions<T> {
    pub fn add_row<'a>(
        &'a mut self,
        f: impl for<'b> FnOnce(&'b mut CreateControlRow<T>) -> &'b mut CreateControlRow<T>,
    ) -> &'a mut Self {
        let mut rw = CreateControlRow::default();
        f(&mut rw);
        self.controls.push(rw.build());
        self
    }

    #[allow(dead_code)]
    pub fn set_timeout<'a>(&'a mut self, timeout: u64) -> &'a mut Self {
        self.timeout = timeout;
        self
    }

    #[allow(dead_code)]
    pub fn set_pre_hook<'a>(&'a mut self, hook: HookFunction<T>) -> &'a mut Self {
        self.pre_hook = Some(hook);
        self
    }

    pub fn set_post_hook<'a>(&'a mut self, hook: HookFunction<T>) -> &'a mut Self {
        self.post_hook = Some(hook);
        self
    }

    pub fn build(self) -> MenuOptions<T> {
        MenuOptions {
            timeout: self.timeout,
            controls: self.controls,
            pre_hook: self.pre_hook,
            post_hook: self.post_hook,
        }
    }
}

pub struct CreateControlRow<T> {
    buttons: Vec<Control<T>>,
}

impl<T> Default for CreateControlRow<T> {
    fn default() -> CreateControlRow<T> {
        CreateControlRow { buttons: vec![] }
    }
}

impl<T> CreateControlRow<T> {
    pub fn add_button<'a>(&'a mut self, button: Control<T>) -> &'a mut Self {
        self.buttons.push(button);
        self
    }

    pub fn build(self) -> ControlRow<T> {
        ControlRow {
            buttons: self.buttons,
        }
    }
}

pub struct MenuOptions<T> {
    timeout: u64,
    controls: Vec<ControlRow<T>>,
    pre_hook: Option<HookFunction<T>>,
    post_hook: Option<HookFunction<T>>,
}

pub struct ControlRow<T> {
    buttons: Vec<Control<T>>,
}

impl<T> ControlRow<T> {
    pub fn action_row(&self) -> serenity::CreateActionRow {
        let mut row = serenity::CreateActionRow::default();
        create_action_row(&mut row, &self.buttons);
        row
    }
}

pub struct Control<T> {
    button: MenuComponent,
    function: ControlFunction<T>,
}

impl<T> Control<T> {
    pub fn new(button: MenuComponent, function: ControlFunction<T>) -> Self {
        Self { button, function }
    }
}

pub type ControlFunction<T> = Arc<
    dyn for<'a> Fn(
            &'a mut Menu<'_, T>,
            &'a Arc<serenity::MessageComponentInteraction>,
        ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + 'a + Send>>
        + Sync
        + Send,
>;

pub type HookFunction<T> = Arc<
    dyn for<'a> Fn(
            &'a mut Menu<'_, T>,
        ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + 'a + Send>>
        + Sync
        + Send,
>;

pub enum MenuComponent {
    ButtonComponent {
        create: serenity::CreateButton,
        id: String,
    },
    #[allow(dead_code)]
    SelectComponent {
        create: serenity::CreateSelectMenu,
        id: String,
    },
}

impl MenuComponent {
    fn id(&self) -> String {
        match self {
            Self::ButtonComponent { create: _, id } => id,
            Self::SelectComponent { create: _, id } => id,
        }
        .clone()
    }

    pub fn button<F>(id: &str, f: F) -> MenuComponent
    where
        F: FnOnce(&mut serenity::CreateButton) -> &mut serenity::CreateButton,
    {
        let mut b = serenity::CreateButton::default();
        Self::ButtonComponent {
            create: f(&mut b).clone(),
            id: id.to_string(),
        }
    }
    #[allow(dead_code)]
    pub fn select<F>(id: &str, f: F) -> MenuComponent
    where
        F: FnOnce(&mut serenity::CreateSelectMenu) -> &mut serenity::CreateSelectMenu,
    {
        let mut b = serenity::CreateSelectMenu::default();
        Self::SelectComponent {
            create: f(&mut b).clone(),
            id: id.to_string(),
        }
    }
}

fn create_action_row<'a, T>(
    a: &'a mut serenity::CreateActionRow,
    buttons: &Vec<Control<T>>,
) -> &'a mut serenity::CreateActionRow {
    for ctrl in buttons {
        match &ctrl.button {
            MenuComponent::ButtonComponent { create, id } => a.create_button(|b| {
                b.clone_from(&create);
                b.custom_id(id)
            }),
            MenuComponent::SelectComponent { create, id } => a.create_select_menu(|sm| {
                sm.clone_from(&create);
                sm.custom_id(id)
            }),
        };
    }
    a
}

pub struct Cursor<'a, T> {
    list: &'a Vec<T>,
    current_index: usize,
}

impl<'a, T> Cursor<'a, T> {
    #[allow(dead_code)]
    pub fn new(list: &'a Vec<T>) -> Self {
        Self {
            list,
            current_index: 0,
        }
    }
    pub fn next(&mut self) -> Option<&'a T> {
        self.current_index = if self.current_index >= self.list.len() - 1 {
            0
        } else {
            self.current_index + 1
        };
        self.list.get(self.current_index)
    }

    pub fn prev(&mut self) -> Option<&'a T> {
        self.current_index = if self.current_index <= 0 {
            self.list.len() - 1
        } else {
            self.current_index - 1
        };
        self.list.get(self.current_index)
    }

    pub fn current(&self) -> Option<&'a T> {
        self.list.get(self.current_index)
    }
}

impl<'a, T> From<&'a Vec<T>> for Cursor<'a, T> {
    fn from(list: &'a Vec<T>) -> Cursor<'a, T> {
        Cursor {
            list,
            current_index: 0,
        }
    }
}

pub async fn generic_select<T, F>(
    m: &mut Menu<'_, T>,
    mci: &Arc<serenity::MessageComponentInteraction>,
    f: impl for<'b, 'c> FnOnce(
        &'b mut serenity::CreateInteractionResponseData<'c>,
        &T,
    ) -> &'b mut serenity::CreateInteractionResponseData<'c>,
) -> Result<(), Error> {
    m.update_response(|mes| f(mes, &m.data), mci).await?;
    Ok(())
}
