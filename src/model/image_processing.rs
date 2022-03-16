use raster;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

const MAX_EMOTE_SIZE: u64 = 256_000; //kb

pub fn reduce_emote_size(img: &PathBuf) -> Result<PathBuf, Error> {
    let mut meta = fs::metadata(img)?;
    let path = match img.to_str() {
        Some(p) => p,
        None => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Couldn't parse path correctly",
            ))
        }
    };

    while meta.len() > MAX_EMOTE_SIZE {
        let image = &mut raster::open(path).unwrap();
        let h = image.height;
        let w = image.width;
        match raster::transform::resize_fit(image, w - w / 4, h - h / 4) {
            Ok(t) => t,
            Err(_) => return Err(Error::new(ErrorKind::Other, "Couldn't resize image")),
        };
        match raster::save(image, path) {
            Ok(t) => t,
            Err(_) => return Err(Error::new(ErrorKind::Other, "Couldn't save image")),
        };
        meta = fs::metadata(img)?;
    }
    return Ok(img.clone());
}
