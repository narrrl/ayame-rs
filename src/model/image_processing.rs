use crate::commands::admin;
use image::{imageops, GenericImageView, ImageFormat};
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use tracing::debug;

pub fn reduce_emote_size(image: &PathBuf) -> Result<PathBuf, Error> {
    let mut meta = fs::metadata(image)?;
    let ext = match image.extension() {
        Some(ext) => ext,
        None => return Err(Error::new(ErrorKind::Other, "No file extension")),
    };

    let ext = match ImageFormat::from_extension(&ext) {
        Some(ext) => ext,
        None => {
            return Err(Error::new(
                ErrorKind::Other,
                format!("couldn't convert {:?} to a valid image extension", &ext),
            ))
        }
    };

    if ext == ImageFormat::Gif {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Can't resize gif's and gif was too large",
        ));
    }

    while meta.len() > *admin::MAX_EMOTE_SIZE {
        let image_buf = match image::open(image) {
            Ok(i) => i,
            Err(_) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("Couldn't open image {:?}", &image),
                ))
            }
        };

        let (width, height) = image_buf.dimensions();

        debug!("Resizing image of {:#?}...", &image_buf.dimensions());
        let image_buf = imageops::resize(
            &image_buf,
            width - (width / 4),
            height - (height / 4),
            imageops::FilterType::Lanczos3,
        );
        if let Err(_) = image_buf.save_with_format(image, ext) {
            return Err(Error::new(
                ErrorKind::Other,
                " Couldn't write image to file",
            ));
        }
        meta = fs::metadata(image)?;
        debug!("new image size of {}", meta.len());
    }
    Ok(image.clone())
}
