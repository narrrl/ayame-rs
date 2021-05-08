use rusqlite::{params, Connection, Error, Result};

#[derive(Debug)]
pub struct Guild {
    id: u64,
    prefix: String,
    volume: u8,
}

impl Guild {
    pub fn get_prefix(&self) -> &str {
        &self.prefix
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn get_volume(&self) -> u8 {
        self.volume
    }

    pub fn set_prefix(&mut self, new_prefix: String, conn: &Connection) -> Result<()> {
        self.prefix = new_prefix;
        update_for(&self, conn)
    }

    pub fn set_volume(&mut self, new_volume: u8, conn: &Connection) -> Result<()> {
        self.volume = new_volume;
        update_for(&self, conn)
    }
}

// connects to the database, creates the guild table and returns the connection
pub fn create_connection() -> Result<Connection, Error> {
    let conn = Connection::open_in_memory()?;

    conn.execute(
        "CREATE TABLE guild (
                id          INTEGER PRIMARY KEY,
                prefix      TEXT NOT NULL,
                volume      INTEGER
                )",
        [],
    )?;
    Ok(conn)
}

// gets the guild manager for a given id from the database
pub fn guild_manager_of(id: u64, conn: &Connection) -> Result<Guild, String> {
    Err(format!("Not implemented"))
}

// update the content in sqlite of given guild.
fn update_for(guild: &Guild, conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO guild (prefix, volume) VALUES (?1, ?2)",
        params![guild.prefix, guild.volume],
    )?;
    Ok(())
}
