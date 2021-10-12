// use std::future::Future;
//
// use serenity::builder::CreateEmbed;
// use serenity::client::Context;
// use serenity::framework::standard::CommandResult;
// use serenity::model::interactions::Interaction;
//
// use crate::model::discord_utils::*;
//
// type SlashResult = std::result::Result<(), ()>;
// type CommandToExecute = fn(Context, Interaction) -> dyn Future<Output = SlashResult>;
//
// struct Command {
//     key: String,
//     execute: CommandToExecute,
// }
//
// struct SlashCommandHandler {
//     commands: Vec<Command>,
// }
//
// async fn invalid(_: Context, _: Interaction) -> SlashResult {
//     Ok(())
// }
//
// impl SlashCommandHandler {
//     fn new() -> Self {
//         let schema = Self { commands: vec![] };
//         schema.commands.push(Command {
//             key: "invalid".to_string(),
//             execute: invalid,
//         });
//         schema
//     }
//     fn add_migration(&mut self, key: String, execute: CommandToExecute) {
//         self.commands.push(Command { key, execute });
//     }
//     fn execute(&self, key: &str) -> SlashResult {
//         Ok(())
//     }
// }
