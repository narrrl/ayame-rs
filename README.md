# nirust
[![Build status](https://github.com/nirusu99/ayame-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/nirusu99/ayame-rs/actions)

A complete (incomplete) rebuild of the [Nirubot](https://github.com/Nirusu99/nirubot) in Rust.

## building
- run `cargo build --release`
- move the executable to your desired final bot directory `mv target/release/ayame-rs to/your/desired/directory/`
- copy the [example config](./example/config.toml) to your bot directory and paste your token \([Where do I get a discord bot token?](https://discord.com/developers/docs/intro)\), the application_id (usually the bots user id) and the prefix (which will trigger the bot in guilds).
- execute the executable with `./ayame-rs`

## running with docker
- create `config.toml`
- build docker image `docker build -t ayame .`
- run docker container with `docker run -d ayame`

## Contact
- **Email**: nils@nirusu.codes

## Invite
[Invite](https://discord.com/api/oauth2/authorize?client_id=702485091842261035&scope=applications.commands+bot&permissions=26909993985) the bot which is hosted by me
