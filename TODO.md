# todo

- [x] database (sqlite probably)
- [ ] bind command
- [ ] bound_channel configuration
    - auto delete messages (to keey the channel clean)
    - ... (ideas go here)
- [ ] bind check for other commands
- [ ] move clusterfuck of hashmaps from data into database
- [ ] rework music commands with binds
- [ ] docker-compose
- [ ] support resize of gifs
    - maybe even some compression/optimization stuff
    - hf julius
- [ ] user prefix or move totally to slash commands only


# bind command

Basically bind a channel on a guild to the bot where all the commands with the `bind_check`
need to be invoked and all responses of bind commands go into the bound channel.

Close integration for music commands for update messages etc. that no other channel gets spammed
by bot messages.
