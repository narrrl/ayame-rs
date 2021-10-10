// TODO:
// use std::future::Future;
//
// use serenity::builder::CreateEmbed;
// use serenity::client::Context;
// use serenity::futures::future::BoxFuture;
// use serenity::model::interactions::Interaction;
//
// use crate::model::discord_utils::*;
//
// type CommandToExecute =
//     Box<dyn Fn(Context, Interaction) -> BoxFuture<'static, CreateEmbed> + Send + 'static>;
//
// struct Command {
//     key: String,
//     execute: CommandToExecute,
// }
//
// struct Schema {
//     commands: Vec<Command>,
// }
//
// async fn invalid(_: Context, _: Interaction) -> CreateEmbed {
//     default_embed()
// }
//
// impl Schema {
//     fn new() -> Self {
//         let schema = Self { commands: vec![] };
//         schema.commands.push(Command {
//             key: "invalid".to_string(),
//             execute: Box::new(invalid),
//         });
//         schema
//     }
//     fn add_migration(&mut self, key: String, execute: CommandToExecute) {
//         self.commands.push(Command { key, execute });
//     }
//     fn execute(&self, key: &str) -> dyn Future<Output = CreateEmbed> {
//         self.commands.iter().find(|c| c.key.eq(key))
//     }
// }
