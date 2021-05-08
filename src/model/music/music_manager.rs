use serenity::model::channel::Message;
use serenity::prelude::Context;

pub fn test() -> String {
    String::from("Test success!")
}

pub fn play(ctx: &Context, msg: &Message) -> Result<String, String> {
    Err(String::from("Not implemented"))
}
