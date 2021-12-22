use async_trait::async_trait;
use serenity::builder::{
    CreateApplicationCommand, CreateApplicationCommands, CreateEmbed, CreateMessage,
};
use serenity::client::Context;
use serenity::framework::standard::CommandResult;
use serenity::http::Http;
use serenity::model::id::ChannelId;
use serenity::model::interactions::application_command::{
    ApplicationCommandInteraction, ApplicationCommandInteractionDataOptionValue,
    ApplicationCommandOptionType,
};
use serenity::model::interactions::InteractionResponseType;
use std::result::Result;
use std::sync::Arc;

use crate::framework;
use crate::model;
use crate::model::discord_utils::{
    check_msg, default_embed, set_defaults_for_error, MusicSelectOptions, SelectMenu,
};
use crate::model::youtube::*;

use super::music::_get_songbird;

#[derive(PartialEq)]
pub enum Scope {
    GUILD,
    GLOBAL,
}

///
/// This trait represents a slash command
///
/// Each slash command is identified by its unique <alias>, which is the also the name for the
/// application command [`ApplicationCommandInteraction`]
///
/// Each slash command needs a create_application function, to setup all slash commands with the
/// discord api
///
/// The run function is where the magic happens for each command
///
#[async_trait]
pub trait SlashCommand {
    fn scope(&self) -> Scope;
    fn alias(&self) -> String;
    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult;
    fn create_application<'a>(
        &self,
        command: &'a mut CreateApplicationCommand,
    ) -> &'a mut CreateApplicationCommand;
}

///
/// The SlashCommandHandler knows all [`SlashCommand`] that can be invoked.
/// This is basically a command pattern
///
pub struct SlashCommandHandler {
    commands: Vec<Box<dyn SlashCommand + Sync + Send>>,
}

///
/// We can add migrations aka [`SlashCommand`] to the [`SlashCommandHandler`]
///
impl SlashCommandHandler {
    pub fn new() -> Self {
        let schema = Self { commands: vec![] };
        schema
    }
    ///
    /// adds a [`SlashCommand`] to the [`SlashCommandHandler`]
    ///
    pub fn add_migration(&mut self, command: Box<dyn SlashCommand + Sync + Send>) {
        self.commands.push(command);
    }
    ///
    /// used to find a [`SlashCommand`] by its alias
    ///
    pub fn find<'a>(&'a self, key: &'a str) -> Option<&'a Box<dyn SlashCommand + Sync + Send>> {
        return self.commands.iter().find(|c| c.alias().eq(key));
    }

    pub fn get_all_create_commands(&self, filter: Scope) -> Vec<CreateApplicationCommand> {
        let mut vec = vec![];
        for slash in self.commands.iter() {
            if filter.eq(&slash.scope()) {
                let mut cmd = CreateApplicationCommand::default();
                cmd.name(slash.alias());
                slash.create_application(&mut cmd);
                vec.push(cmd);
            }
        }
        vec
    }

    pub fn get_all_aliases<'a>(&'a self, filter: Scope) -> Vec<String> {
        let mut vec = vec![];
        for slash in self.commands.iter() {
            if filter.eq(&slash.scope()) {
                vec.push(slash.alias());
            }
        }
        vec
    }
}
///
/// Stores all [`CreateApplicationCommand`] in a [`CreateApplicationCommands`] to send them to
/// discord with one singe invocation
///
pub fn create_commands(commands: &mut CreateApplicationCommands) -> &mut CreateApplicationCommands {
    // we iterate over every command
    for slash in get_slash_handler().commands.iter() {
        if let Scope::GLOBAL = slash.scope() {
            commands.create_application_command(|command| {
                // get the command name from the alias
                command.name(slash.alias());
                // invoke the method to create the whole command
                slash.create_application(command)
            });
        }
    }
    // return all [`CreateApplicationCommand`] as [`CreateApplicationCommands`]
    commands
}

///
/// Gets a [`SlashCommandHandler`] with all [`SlashCommand`]'s
///
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
    handler.add_migration(Box::new(Search));
    handler
}

// our [`SlashCommand`]'s
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
struct Search;

//
// **All [`SlashCommand`]'s gets implemented below here!**
//

#[async_trait]
impl SlashCommand for Play {
    fn scope(&self) -> Scope {
        Scope::GUILD
    }
    fn alias(&self) -> String {
        String::from("play")
    }

    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult {
        let options = _get_options(&command, 0)?;
        if let ApplicationCommandInteractionDataOptionValue::String(text) = options {
            let guild_id = command.guild_id.unwrap();
            let guild = guild_id.to_guild_cached(&ctx.cache).await.unwrap();
            let manager = _get_songbird(&ctx).await;
            let channel_id = command.channel_id;
            let mut already_responded = false;
            // check if bot isn't connected just yet
            if manager.get(guild_id).is_none() {
                let author_id = command.user.id;
                let result = framework::music::join(&ctx, &guild, author_id).await;
                // if the bot couldn't join, we can stop the complete command
                let is_error = result.is_err();
                _send_response(&ctx.http, result, &command).await;
                already_responded = true;
                if is_error {
                    // end command
                    return Ok(());
                }
            }
            // we execute the play command
            let result = framework::music::play(
                &ctx,
                &guild,
                &channel_id,
                &command.user.id,
                text.to_string(),
            )
            .await;
            // check if the application command already responded
            if already_responded {
                // if yes, we simply send a messag
                _send_message(&ctx.http, result, channel_id).await;
            } else {
                // else send the response
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
    fn scope(&self) -> Scope {
        Scope::GLOBAL
    }
    fn alias(&self) -> String {
        String::from("mock")
    }

    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult {
        let options = _get_options(&command, 0)?;

        if let ApplicationCommandInteractionDataOptionValue::String(text) = options {
            // mock text
            let text = model::mock_text(&text);
            // send mock'ed text as response
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
    fn scope(&self) -> Scope {
        Scope::GLOBAL
    }
    fn alias(&self) -> String {
        String::from("invite")
    }

    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult {
        let embed = framework::general::invite(&ctx.http).await;
        _send_response(&ctx.http, Ok(embed), &command).await;
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
    fn scope(&self) -> Scope {
        Scope::GUILD
    }
    fn alias(&self) -> String {
        String::from("join")
    }

    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult {
        let guild_id = command.guild_id.unwrap();
        let guild = guild_id.to_guild_cached(&ctx.cache).await.unwrap();
        let author_id = command.user.id;
        let result = framework::music::join(&ctx, &guild, author_id).await;
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
    fn scope(&self) -> Scope {
        Scope::GUILD
    }
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
    fn scope(&self) -> Scope {
        Scope::GUILD
    }
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
    fn scope(&self) -> Scope {
        Scope::GUILD
    }
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
    fn scope(&self) -> Scope {
        Scope::GUILD
    }
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
    fn scope(&self) -> Scope {
        Scope::GUILD
    }
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
    fn scope(&self) -> Scope {
        Scope::GUILD
    }
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
    fn scope(&self) -> Scope {
        Scope::GUILD
    }
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

#[async_trait]
impl SlashCommand for Search {
    fn scope(&self) -> Scope {
        Scope::GUILD
    }
    fn alias(&self) -> String {
        String::from("search")
    }

    async fn run(&self, ctx: Context, command: ApplicationCommandInteraction) -> CommandResult {
        let options = _get_options(&command, 0)?;
        if let ApplicationCommandInteractionDataOptionValue::String(search_term) = options {
            let guild_id = command.guild_id.unwrap();
            let guild = guild_id.to_guild_cached(&ctx.cache).await.unwrap();
            let manager = _get_songbird(&ctx).await;
            let channel_id = command.channel_id;
            let author_id = command.user.id;

            // we get the bot config, for the youtube api key
            let conf = &crate::CONFIG;
            // create a new [`YoutubeSearch`]
            let mut req = YoutubeSearch::new(&conf.youtube_api_key());
            // set the retrived video amount to 5
            // and set the filter to only retrive videos
            req.set_amount(5).set_filter(Type::VIDEO);

            // we start the search with the user input
            let res = req.search(search_term).await?;

            // if we got nothing, we inform the user and end the command
            if res.results().is_empty() {
                let mut e = default_embed();
                set_defaults_for_error(&mut e, "nothing found");
                _send_response(&ctx.http, Ok(e), &command).await;
                return Ok(());
            }

            // we now create the pages of the different search results
            let mut pages = vec![];
            // we need a first page, to respond to the interaction
            let mut first_page = default_embed();
            for (index, result) in res.results().iter().enumerate() {
                let mut e = default_embed();
                e.image(result.thumbnail().url());
                e.field(
                    "Title:",
                    &format!("[{}]({})", result.title(), result.url()),
                    false,
                );
                e.field(
                    "Channel:",
                    &format!("[{}]({})", result.channel_name(), result.channel_url()),
                    false,
                );

                e.field(
                    "Published:",
                    result
                        .time_published()
                        .format("%H:%M, %a %Y-%m-%d")
                        .to_string(),
                    false,
                );
                let mut mes = CreateMessage::default();
                // we use the top search result as the first page
                if index == 0 {
                    first_page = e.clone();
                }
                mes.set_embed(e);
                pages.push(mes);
            }
            // we send the first page
            _send_response(&ctx.http, Ok(first_page), &command).await;

            // get the [`Message`] from the response
            let msg = command.get_interaction_response(&ctx.http).await?;

            // we create the options for the select screen
            let mut options = MusicSelectOptions::default();
            // where we set the response message as the message to edit
            options.set_message(msg);

            // we now create the menu
            let menu = SelectMenu::new(&ctx, &author_id, &channel_id, &pages, options);

            // which then gets us the selected song as a index
            // because the index of the pages is equal to the index of the video results
            let index = match menu.run().await {
                Ok((index, _)) => index,
                Err(_) => return Ok(()),
            };

            // thats why we can just get the result the user want from the
            // [`YoutubeResponse`] and unwrap it
            let choice = res.results().get(index).unwrap();

            // now we check if the bot is not connected yet
            if manager.get(guild_id).is_none() {
                let author_id = command.user.id;
                let result = framework::music::join(&ctx, &guild, author_id).await;
                if result.is_err() {
                    return Ok(());
                }
            }
            // and then we queue the desired song
            let result =
                framework::music::play(&ctx, &guild, &channel_id, &command.user.id, choice.url())
                    .await;
            _send_message(&ctx.http, result, channel_id).await;
        }
        Ok(())
    }

    fn create_application<'a>(
        &self,
        command: &'a mut CreateApplicationCommand,
    ) -> &'a mut CreateApplicationCommand {
        command
            .description("Searches for music on Youtube")
            .create_option(|option| {
                option
                    .name("query")
                    .description("The search term")
                    .kind(ApplicationCommandOptionType::String)
                    .required(true)
            });
        command
    }
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

async fn _make_response(
    http: &Arc<Http>,
    embed: CreateEmbed,
    command: &ApplicationCommandInteraction,
) {
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

async fn _send_response(
    http: &Arc<Http>,
    result: Result<CreateEmbed, String>,
    command: &ApplicationCommandInteraction,
) {
    match result {
        Ok(embed) => {
            _make_response(http, embed, command).await;
        }
        Err(why) => {
            let mut embed = default_embed();
            set_defaults_for_error(&mut embed, &why);
            _make_response(http, embed, command).await;
        }
    };
}

fn _get_options<'a>(
    command: &'a ApplicationCommandInteraction,
    index: usize,
) -> crate::error::Result<&'a ApplicationCommandInteractionDataOptionValue> {
    match command.data.options.get(index) {
        Some(option) => match &option.resolved {
            Some(res) => return Ok(&res),
            None => return Err(crate::error::Error::from("expected user input")),
        },
        None => return Err(crate::error::Error::from("expected user input")),
    }
}
