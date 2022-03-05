use poise::serenity_prelude as serenity;
use std::fmt::Write as _;

/// A shared instance of this struct is available across all events and framework commands
struct Data {
    command_counter: std::sync::Mutex<std::collections::HashMap<String, u64>>,
}
/// This Error type is used throughout all commands and callbacks
type Error = Box<dyn std::error::Error + Send + Sync>;

/// This type alias will save us some typing, because the Context type is needed often
type Context<'a> = poise::Context<'a, Data, Error>;

async fn event_listener(
    _ctx: &serenity::Context,
    event: &poise::Event<'_>,
    _framework: &poise::Framework<Data, Error>,
    _user_data: &Data,
) -> Result<(), Error> {
    match event {
        poise::Event::Ready { data_about_bot } => {
            println!("{} is connected!", data_about_bot.user.name)
        }
        _ => {}
    }

    Ok(())
}

// The framework provides built-in help functionality for you to use.
// You just have to set the metadata of the command like descriptions, to fit with the rest of your
// bot. The actual help text generation is delegated to poise
/// Show a help menu
#[poise::command(prefix_command, slash_command)]
async fn help(
    ctx: Context<'_>,
    #[description = "Command to display specific information about"] command: Option<String>,
) -> Result<(), Error> {
    let config = poise::builtins::HelpConfiguration {
        extra_text_at_bottom: "\
Hello! こんにちは！Hola! Bonjour! 您好! 안녕하세요~
If you want more information about a specific command, just pass the command as argument.",
        ..Default::default()
    };

    poise::builtins::help(ctx, command.as_deref(), config).await?;

    Ok(())
}

/// Register slash commands in this guild or globally
///
/// Run with no arguments to register in guild, run with argument "global" to register globally.
#[poise::command(prefix_command, hide_in_help)]
async fn register(ctx: Context<'_>, #[flag] global: bool) -> Result<(), Error> {
    poise::builtins::register_application_commands(ctx, global).await?;

    Ok(())
}

async fn pre_command(ctx: Context<'_>) {
    println!(
        "Got command '{}' by user '{}'",
        ctx.command().name,
        ctx.author().name
    );

    // Increment the number of times this command has been run once. If
    // the command's name does not exist in the counter, add a default
    // value of 0.
    let mut command_counter = ctx.data().command_counter.lock().unwrap();
    let entry = command_counter
        .entry(ctx.command().name.to_string())
        .or_insert(0);
    *entry += 1;
}

async fn post_command(ctx: Context<'_>) {
    println!("Processed command '{}'", ctx.command().name);
}

// TODO: unify the command checks in poise::FrameworkOptions and then implement a command check here
// with this in it:
// ```
// true // if `check` returns false, command processing doesn't happen.
// ```

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Command { error, ctx } => {
            println!(
                "Command '{}' returned error {:?}",
                ctx.command().name,
                error
            );
        }
        poise::FrameworkError::Listener { error, event } => {
            println!(
                "Listener returned error during {:?} event: {:?}",
                event.name(),
                error
            );
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}

// INFO: Poise currently does not support callbacks for these events
/*#[hook]
async fn unknown_command(_ctx: &Context, _msg: &Message, unknown_command_name: &str) {
    println!("Could not find command named '{}'", unknown_command_name);
}
#[hook]
async fn normal_message(_ctx: Context<'_>) {
    println!("Message is not a command '{}'", msg.content);
}*/

// INFO: Currently not applicable because poise doesn't have cooldowns
/*#[hook]
async fn delay_action(ctx: Context<'_>) {
    // You may want to handle a Discord rate limit if this fails.
    let _ = msg.react(ctx, '⏱').await;
}
#[hook]
async fn dispatch_error(ctx: Context<'_>, error: DispatchError) {
    if let DispatchError::Ratelimited(info) = error {
        // We notify them only once.
        if info.is_first_try {
            let _ = msg
                .channel_id
                .say(&ctx.http, &format!("Try this again in {} seconds.", info.as_secs()))
                .await;
        }
    }
}
// You can construct a hook without the use of a macro, too.
// This requires some boilerplate though and the following additional import.
use serenity::{futures::future::BoxFuture, FutureExt};
fn _dispatch_error_no_macro<'fut>(
    ctx: &'fut mut Context,
    msg: &'fut Message,
    error: DispatchError,
) -> BoxFuture<'fut, ()> {
    async move {
        if let DispatchError::Ratelimited(info) = error {
            if info.is_first_try {
                let _ = msg
                    .channel_id
                    .say(&ctx.http, &format!("Try this again in {} seconds.", info.as_secs()))
                    .await;
            }
        };
    }
    .boxed()
}*/

#[tokio::main]
async fn main() {
    let options = poise::FrameworkOptions {
        commands: vec![
            // The `#[poise::command(prefix_command, slash_command)]` macro transforms the function into
            // `fn() -> poise::Command`.
            // Therefore, you need to call the command function without any arguments to get the
            // command definition instance to pass to the framework
            help(),
            // This function registers slash commands on Discord. When you change something about a
            // command signature, for example by changing its name, adding or removing parameters, or
            // changing a parameter type, you should call this function.
            register(),
            about(),
            am_i_admin(),
            say(),
            commands(),
            ping(),
            latency(),
            some_long_command(),
            poise::Command {
                // A command can have sub-commands, just like in command lines tools.
                // Imagine `cargo help` and `cargo help run`.
                subcommands: vec![sub()],
                ..upper_command()
            },
            bird(),
            cat(),
            dog(),
            multiply(),
            slow_mode(),
        ],
        listener: |ctx, event, framework, user_data| {
            Box::pin(event_listener(ctx, event, framework, user_data))
        },
        on_error: |error| Box::pin(on_error(error)),
        // Set a function to be called prior to each command execution. This
        // provides all context of the command that would also be passed to the actual command code
        pre_command: |ctx| Box::pin(pre_command(ctx)),
        // Similar to `pre_command`, except will be called directly _after_
        // command execution.
        post_command: |ctx| Box::pin(post_command(ctx)),

        // Options specific to prefix commands, i.e. commands invoked via chat messages
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some(String::from("~")),

            mention_as_prefix: false,
            // An edit tracker needs to be supplied here to make edit tracking in commands work
            edit_tracker: Some(poise::EditTracker::for_timespan(
                std::time::Duration::from_secs(3600 * 3),
            )),
            ..Default::default()
        },

        ..Default::default()
    };

    // The Framework builder will automatically retrieve the bot owner and application ID via the
    // passed token, so that information need not be passed here
    poise::Framework::build()
        // Configure the client with your Discord bot token in the environment.
        .token(std::env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in the environment"))
        .options(options)
        .user_data_setup(|_ctx, _data_about_bot, _framework| {
            Box::pin(async move {
                Ok(Data {
                    command_counter: std::sync::Mutex::new(std::collections::HashMap::new()),
                })
            })
        })
        .run()
        .await
        .expect("Client error");

    // INFO: currently not supported by poise
    /*
    // Set a function that's called whenever an attempted command-call's
    // command could not be found.
    .unrecognised_command(unknown_command)
    // Set a function that's called whenever a message is not a command.
    .normal_message(normal_message)
    // Set a function that's called whenever a command's execution didn't complete for one
    // reason or another. For example, when a user has exceeded a rate-limit or a command
    // can only be performed by the bot owner.
    .on_dispatch_error(dispatch_error)
    // Can't be used more than once per 5 seconds:
    .bucket("emoji", |b| b.delay(5)).await
    // Can't be used more than 2 times per 30 seconds, with a 5 second delay applying per channel.
    // Optionally `await_ratelimits` will delay until the command can be executed instead of
    // cancelling the command invocation.
    .bucket("complicated", |b| b.limit(2).time_span(30).delay(5)
        // The target each bucket will apply to.
        .limit_for(LimitedFor::Channel)
        // The maximum amount of command invocations that can be delayed per target.
        // Setting this to 0 (default) will never await/delay commands and cancel the invocation.
        .await_ratelimits(1)
        // A function to call when a rate limit leads to a delay.
        .delay_action(delay_action)
    ).await
    */
}
