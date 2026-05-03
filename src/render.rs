use crate::layout::Placement;
use anyhow::{Context, Result};
use image::{ImageReader, Rgb, RgbImage, imageops};
use std::path::Path;

/// Composite all placements onto a white canvas and return the result.
pub fn draw(placements: &[Placement], canvas_w: u32, canvas_h: u32) -> Result<RgbImage> {
    let mut canvas = white_canvas(canvas_w, canvas_h);
    for p in placements {
        paste(&mut canvas, p)?;
    }
    Ok(canvas)
}

fn white_canvas(w: u32, h: u32) -> RgbImage {
    RgbImage::from_pixel(w, h, Rgb([255, 255, 255]))
}

fn paste(canvas: &mut RgbImage, p: &Placement) -> Result<()> {
    if p.w == 0 || p.h == 0 {
        return Ok(());
    }
    let src = load_raw(&p.path)
        .with_context(|| format!("load {:?}", p.path))?;
    let scaled = imageops::resize(&src, p.w, p.h, imageops::FilterType::Lanczos3);
    imageops::replace(canvas, &scaled, p.x as i64, p.y as i64);
    Ok(())
}

/// Decode raw pixel data from `path` without applying EXIF orientation.
///
/// `image::open` / `ImageReader::decode` return raw pixel data and do not
/// call `DynamicImage::apply_orientation`, so the bytes on disk are used
/// exactly as stored.  This function makes that contract explicit: callers
/// must never pass the result through `apply_orientation`.
fn load_raw(path: &Path) -> Result<RgbImage> {
    Ok(ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?
        .into_rgb8())
}
