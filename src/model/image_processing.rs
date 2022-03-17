use raster;
use std::fs;
use std::path::PathBuf;

use crate::Error;

const MAX_EMOTE_SIZE: u64 = 256_000; //kb

pub fn reduce_emote_size<'a>(img: &'a PathBuf) -> Result<&'a PathBuf, Error> {
    let mut meta = fs::metadata(img)?;
    let path = img
        .to_str()
        .ok_or_else(|| Error::Failure("couldn't parse path to image"))?;

    while meta.len() > MAX_EMOTE_SIZE {
        let mut image = match raster::open(path) {
            Ok(image) => image,
            Err(_) => return Err(Error::Failure("couldn't open image")),
        };
        let h = image.height;
        let w = image.width;
        if let Err(_) = raster::transform::resize_fit(&mut image, w - w / 4, h - h / 4) {
            return Err(Error::Failure("couldn't resize image"));
        }
        if let Err(_) = raster::save(&mut image, path) {
            return Err(Error::Failure("couldn't save image"));
        }
        meta = fs::metadata(img)?;
    }
    return Ok(&img);
}
