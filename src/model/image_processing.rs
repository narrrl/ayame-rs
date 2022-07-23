use std::fs;
use std::path::PathBuf;

use crate::Error;

const MAX_EMOTE_SIZE: u64 = 256_000; //kb

pub fn reduce_emote_size<'a>(img: &'a PathBuf) -> Result<&'a PathBuf, Error> {
    return Ok(&img);
}
