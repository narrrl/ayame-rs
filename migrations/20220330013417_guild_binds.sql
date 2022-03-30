-- Add migration script here
CREATE TABLE guild_bind {
    guild_id INTEGER NOT NULL PRIMARY KEY,
    bind_id INTEGER NOT NULL,
    delete_messages BOOLEAN NOT NULL,
}
