# nirust
A complete (incomplete) rebuild of the [Nirubot](https://github.com/Nirusu99/nirubot) in Rust.

## building
- run `cargo build --release`
- move the executable to your desired final bot directory `mv target/release/ayame-rs to/your/desired/directory/`
- copy the [example config](./example/config.toml) to your bot directory and paste your token \([Where do I get a discord bot token?](https://discord.com/developers/docs/intro)\), the application_id (usually the bots user id) and the prefix (which will trigger the bot in guilds).
- execute the executable with `./ayame-rs`

## building and running in GNU/Linux
- run `./run.sh start`, the script will ask for the credentials
- run `./run.sh stop` to stop the bot
- run `./run.sh update` to update the bot

## Contact
- **Email**: nils@nirusu.codes

## Invite
[Invite](https://discord.com/api/oauth2/authorize?client_id=702485091842261035&scope=applications.commands+bot&permissions=26909993985) the bot which is hosted by me
