use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    error::{COULDNT_GET_MSG, UNKNOWN_RESPONSE},
    Context, Error,
};
use poise::{serenity_prelude as serenity, CreateReply};

pub struct Menu<'a, T> {
    pub ctx: &'a Context<'a>,
    options: MenuOptions<T>,
    pub msg_id: Option<serenity::MessageId>,
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
            msg_id: None,
            is_runnig: true,
        }
    }

    pub async fn run(
        &mut self,
        f: impl for<'b, 'c> FnOnce(&'b mut CreateReply<'c>) -> &'b mut CreateReply<'c>,
    ) -> Result<(), Error> {
        let msg_id = self.send_msg(f).await?;
        self.msg_id = Some(msg_id);
        while let Some(mci) = serenity::CollectComponentInteraction::new(self.ctx.discord())
            .author_id(self.ctx.author().id)
            .channel_id(self.ctx.channel_id())
            .timeout(std::time::Duration::from_secs(self.options.timeout))
            .await
        {
            let action = &self
                .options
                .controls
                .iter()
                .map(|row| &row.buttons)
                .flatten()
                .find(|ctrl| ctrl.button.id() == mci.data.custom_id)
                .ok_or_else(|| Error::Failure(UNKNOWN_RESPONSE))?;
            let func = Arc::clone(&action.function);

            // run function of button/context
            func(self, &mci).await?;

            // check if responded
            if let Err(_) = mci.get_interaction_response(&self.ctx.discord().http).await {
                // else respond with empty event
                mci.create_interaction_response(&self.ctx.discord(), |ir| {
                    ir.kind(serenity::InteractionResponseType::DeferredUpdateMessage)
                })
                .await?;
            }

            if !self.is_runnig {
                break;
            }
        }
        Ok(())
    }
    pub async fn send_msg(
        &self,
        f: impl for<'b, 'c> FnOnce(&'b mut CreateReply<'c>) -> &'b mut CreateReply<'c>,
    ) -> Result<serenity::MessageId, Error> {
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
            .await?
            .ok_or_else(|| Error::Failure(COULDNT_GET_MSG))?;
        let msg_id = handle.message().await?.id;
        Ok(msg_id)
    }

    pub async fn update_response(
        &self,
        f: impl for<'b, 'c> FnOnce(
            &'b mut serenity::CreateInteractionResponseData,
        ) -> &'b mut serenity::CreateInteractionResponseData,
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
}

impl<T> Default for CreateMenuOptions<T> {
    fn default() -> CreateMenuOptions<T> {
        CreateMenuOptions {
            timeout: 120,
            controls: vec![],
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
    pub fn build(self) -> MenuOptions<T> {
        MenuOptions {
            timeout: self.timeout,
            controls: self.controls,
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

pub enum MenuComponent {
    ButtonComponent {
        create: serenity::CreateButton,
        id: String,
    },
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
