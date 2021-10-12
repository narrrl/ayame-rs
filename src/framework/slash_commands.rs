use async_trait::async_trait;
use serenity::builder::{CreateApplicationCommand, CreateApplicationCommands, CreateEmbed};
use serenity::client::Context;
use serenity::framework::standard::CommandResult;
use serenity::http::Http;
use serenity::model::id::ChannelId;
use serenity::model::interactions::application_command::{
    ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
    ApplicationCommandOptionType,
};
use serenity::model::interactions::InteractionResponseType;
use serenity::Result as SerenityResult;
use std::result::Result;
use std::sync::Arc;
use tracing::error;

use crate::framework;
use crate::model;
use crate::model::discord_utils::{default_embed, set_defaults_for_error};

use super::music::_get_songbird;

#[async_trait]
pub trait SlashCommand {
    fn alias(&self) -> String;
    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult;
    fn create_application<'a>(
        &self,
        command: &'a mut CreateApplicationCommand,
    ) -> &'a mut CreateApplicationCommand;
}

pub struct SlashCommandHandler {
    commands: Vec<Box<dyn SlashCommand + Sync + Send>>,
}

impl SlashCommandHandler {
    pub fn new() -> Self {
        let schema = Self { commands: vec![] };
        schema
    }
    pub fn add_migration(&mut self, command: Box<dyn SlashCommand + Sync + Send>) {
        self.commands.push(command);
    }
    pub fn find<'a>(&'a self, key: &'a str) -> Option<&'a Box<dyn SlashCommand + Sync + Send>> {
        return self.commands.iter().find(|c| c.alias().eq(key));
    }
}

pub fn get_slash_handler() -> SlashCommandHandler {
    let mut handler = SlashCommandHandler::new();
    handler.add_migration(Box::new(Play));
    handler.add_migration(Box::new(Join));
    handler.add_migration(Box::new(Invite));
    handler.add_migration(Box::new(Mock));
    handler.add_migration(Box::new(Leave));
    handler.add_migration(Box::new(Skip));
    handler.add_migration(Box::new(Mute));
    handler.add_migration(Box::new(Deafen));
    handler.add_migration(Box::new(Playing));
    handler.add_migration(Box::new(Pause));
    handler.add_migration(Box::new(Resume));
    handler
}

pub fn create_commands(commands: &mut CreateApplicationCommands) -> &mut CreateApplicationCommands {
    for slash in get_slash_handler().commands.iter() {
        commands.create_application_command(|command| {
            command.name(slash.alias());
            slash.create_application(command)
        });
    }
    commands
}

struct Play;
struct Join;
struct Invite;
struct Mock;
struct Leave;
struct Skip;
struct Mute;
struct Deafen;
struct Playing;
struct Pause;
struct Resume;

#[async_trait]
impl SlashCommand for Play {
    fn alias(&self) -> String {
        String::from("play")
    }

    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult {
        let options = command
            .data
            .options
            .get(0)
            .expect("Expected user option")
            .resolved
            .as_ref()
            .expect("Expected String object");
        if let ApplicationCommandInteractionDataOptionValue::String(text) = options {
            let guild_id = command.guild_id.unwrap();
            let guild = guild_id.to_guild_cached(&ctx.cache).await.unwrap();
            let manager = _get_songbird(&ctx).await;
            let channel_id = command.channel_id;
            // TODO: better auto join
            let mut already_responded = false;
            if manager.get(guild_id).is_none() {
                let author_id = command.user.id;
                let result = framework::music::join(&ctx, &guild, author_id, channel_id).await;
                let is_error = result.is_err();
                _send_response(&ctx.http, result, &command).await;
                already_responded = true;
                if is_error {
                    return Ok(());
                }
            }
            let result = framework::music::play(&ctx, &guild, text.to_string()).await;
            if already_responded {
                _send_message(&ctx.http, result, channel_id).await;
            } else {
                _send_response(&ctx.http, result, &command).await;
            }
        }
        Ok(())
    }

    fn create_application<'a>(
        &self,
        command: &'a mut CreateApplicationCommand,
    ) -> &'a mut CreateApplicationCommand {
        command
            .description("Play some music")
            .create_option(|option| {
                option
                    .name("link")
                    .description("The link that should be played")
                    .kind(ApplicationCommandOptionType::String)
                    .required(true)
            });
        command
    }
}

#[async_trait]
impl SlashCommand for Mock {
    fn alias(&self) -> String {
        String::from("mock")
    }

    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult {
        let options = command
            .data
            .options
            .get(0)
            .expect("Expected user option")
            .resolved
            .as_ref()
            .expect("Expected String object");

        if let ApplicationCommandInteractionDataOptionValue::String(text) = options {
            let text = model::mock_text(&text);
            check_msg(
                command
                    .create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| message.content(text))
                    })
                    .await,
            );
        }
        Ok(())
    }

    fn create_application<'a>(
        &self,
        command: &'a mut CreateApplicationCommand,
    ) -> &'a mut CreateApplicationCommand {
        command
            .description("Converts your message to random upper and lower case")
            .create_option(|option| {
                option
                    .name("message")
                    .description("Your message that gets converted")
                    .kind(ApplicationCommandOptionType::String)
                    .required(true)
            });
        command
    }
}

#[async_trait]
impl SlashCommand for Invite {
    fn alias(&self) -> String {
        String::from("invite")
    }

    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult {
        let embed = framework::general::invite(&ctx.http).await;
        check_msg(command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(serenity::model::interactions::InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|b| b.add_embed(embed))
            })
            .await);
        Ok(())
    }

    fn create_application<'a>(
        &self,
        command: &'a mut CreateApplicationCommand,
    ) -> &'a mut CreateApplicationCommand {
        command.description("Invite the bot to your server");
        command
    }
}
#[async_trait]
impl SlashCommand for Join {
    fn alias(&self) -> String {
        String::from("join")
    }

    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult {
        let guild_id = command.guild_id.unwrap();
        let guild = guild_id.to_guild_cached(&ctx.cache).await.unwrap();
        let author_id = command.user.id;
        let channel_id = command.channel_id;
        let result = framework::music::join(&ctx, &guild, author_id, channel_id).await;
        _send_response(&ctx.http, result, &command).await;
        Ok(())
    }

    fn create_application<'a>(
        &self,
        command: &'a mut CreateApplicationCommand,
    ) -> &'a mut CreateApplicationCommand {
        command.description("Joins the current channel");
        command
    }
}

#[async_trait]
impl SlashCommand for Leave {
    fn alias(&self) -> String {
        String::from("leave")
    }

    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult {
        let guild_id = command.guild_id.unwrap();
        let result = framework::music::leave(&ctx, guild_id).await;
        _send_response(&ctx.http, result, &command).await;
        Ok(())
    }

    fn create_application<'a>(
        &self,
        command: &'a mut CreateApplicationCommand,
    ) -> &'a mut CreateApplicationCommand {
        command.description("Leaves the current channel");
        command
    }
}

#[async_trait]
impl SlashCommand for Skip {
    fn alias(&self) -> String {
        String::from("skip")
    }

    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult {
        let guild_id = command.guild_id.unwrap();
        let result = framework::music::skip(&ctx, guild_id).await;
        _send_response(&ctx.http, result, &command).await;
        Ok(())
    }

    fn create_application<'a>(
        &self,
        command: &'a mut CreateApplicationCommand,
    ) -> &'a mut CreateApplicationCommand {
        command.description("skips the current song");
        command
    }
}

#[async_trait]
impl SlashCommand for Mute {
    fn alias(&self) -> String {
        String::from("mute")
    }

    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult {
        let guild_id = command.guild_id.unwrap();
        let result = framework::music::mute(&ctx, guild_id).await;
        _send_response(&ctx.http, result, &command).await;
        Ok(())
    }

    fn create_application<'a>(
        &self,
        command: &'a mut CreateApplicationCommand,
    ) -> &'a mut CreateApplicationCommand {
        command.description("mutes the bot");
        command
    }
}

#[async_trait]
impl SlashCommand for Deafen {
    fn alias(&self) -> String {
        String::from("deafen")
    }

    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult {
        let guild_id = command.guild_id.unwrap();
        let result = framework::music::deafen(&ctx, guild_id).await;
        _send_response(&ctx.http, result, &command).await;
        Ok(())
    }

    fn create_application<'a>(
        &self,
        command: &'a mut CreateApplicationCommand,
    ) -> &'a mut CreateApplicationCommand {
        command.description("deafens the bot");
        command
    }
}

#[async_trait]
impl SlashCommand for Playing {
    fn alias(&self) -> String {
        String::from("playing")
    }

    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult {
        let guild_id = command.guild_id.unwrap();
        let result = framework::music::now_playing(&ctx, guild_id).await;
        _send_response(&ctx.http, result, &command).await;
        Ok(())
    }

    fn create_application<'a>(
        &self,
        command: &'a mut CreateApplicationCommand,
    ) -> &'a mut CreateApplicationCommand {
        command.description("shows the current playing track");
        command
    }
}

#[async_trait]
impl SlashCommand for Pause {
    fn alias(&self) -> String {
        String::from("pause")
    }

    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult {
        let guild_id = command.guild_id.unwrap();
        let result = framework::music::play_pause(&ctx, guild_id).await;
        _send_response(&ctx.http, result, &command).await;
        Ok(())
    }

    fn create_application<'a>(
        &self,
        command: &'a mut CreateApplicationCommand,
    ) -> &'a mut CreateApplicationCommand {
        command.description("pauses/resumes current playing track");
        command
    }
}
#[async_trait]
impl SlashCommand for Resume {
    fn alias(&self) -> String {
        String::from("resume")
    }

    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult {
        let guild_id = command.guild_id.unwrap();
        let result = framework::music::play_pause(&ctx, guild_id).await;
        _send_response(&ctx.http, result, &command).await;
        Ok(())
    }

    fn create_application<'a>(
        &self,
        command: &'a mut CreateApplicationCommand,
    ) -> &'a mut CreateApplicationCommand {
        command.description("pauses/resumes current playing track");
        command
    }
}

fn check_msg(result: SerenityResult<()>) {
    if let Err(why) = result {
        error!("failed: {:?}", why);
    }
}

async fn _send_response(
    http: &Arc<Http>,
    result: Result<CreateEmbed, String>,
    command: &ApplicationCommandInteraction,
) {
    match result {
        Ok(embed) => {
            check_msg(
                command
                    .create_interaction_response(http, |response| {
                        response
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| message.add_embed(embed))
                    })
                    .await,
            );
        }
        Err(why) => {
            let mut embed = default_embed();
            set_defaults_for_error(&mut embed, &why);
            check_msg(
                command
                    .create_interaction_response(http, |response| {
                        response
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| message.add_embed(embed))
                    })
                    .await,
            );
        }
    };
}

async fn _send_message(
    http: &Arc<Http>,
    result: Result<CreateEmbed, String>,
    channel_id: ChannelId,
) {
    match result {
        Ok(embed) => {
            model::discord_utils::check_msg(
                channel_id.send_message(http, |m| m.set_embed(embed)).await,
            );
        }
        Err(why) => {
            let mut embed = default_embed();
            set_defaults_for_error(&mut embed, &why);
            model::discord_utils::check_msg(
                channel_id.send_message(http, |m| m.set_embed(embed)).await,
            );
        }
    };
}
