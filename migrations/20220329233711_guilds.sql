-- Add migration script here
CREATE TABLE guild_binds {
    guild_id INTEGER NOT NULL PRIMARY KEY,
    bind_id INTEGER NOT NULL,
    delete_messages BOOLEAN NOT NULL,
};
CREATE TABLE guild {
    guild_id INTEGER NOT NULL PRIMARY KEY,
    prefix TEXT NOT NULL,
};
