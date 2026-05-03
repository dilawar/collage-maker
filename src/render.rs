use crate::effects::{self, EffectOptions};
use crate::layout::Placement;
use anyhow::{Context, Result};
use image::{ImageReader, Rgb, RgbImage, RgbaImage, imageops};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;

/// Composite all placements onto a white canvas and return the result.
pub fn draw(
    placements: &[Placement],
    canvas_w: u32,
    canvas_h: u32,
    opts: &EffectOptions,
) -> Result<RgbImage> {
    let mut canvas = white_canvas(canvas_w, canvas_h);
    let pb = render_bar(placements.len());
    for p in placements {
        pb.set_message(
            p.path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        );
        paste(&mut canvas, p, opts)?;
        pb.inc(1);
    }
    pb.finish_and_clear();
    Ok(canvas)
}

fn white_canvas(w: u32, h: u32) -> RgbImage {
    RgbImage::from_pixel(w, h, Rgb([255, 255, 255]))
}

fn paste(canvas: &mut RgbImage, p: &Placement, opts: &EffectOptions) -> Result<()> {
    if p.w == 0 || p.h == 0 {
        return Ok(());
    }

    let needs_effects = p.rotation.is_some() || opts.active();

    if !needs_effects {
        // Fast path: pure RGB, no alpha compositing needed.
        let src = load_raw(&p.path).with_context(|| format!("load {:?}", p.path))?;
        let scaled = imageops::resize(&src, p.w, p.h, imageops::FilterType::Lanczos3);
        imageops::replace(canvas, &scaled, p.x as i64, p.y as i64);
        return Ok(());
    }

    // RGBA path: rotation + alpha-mask effects.
    let src = load_rgba(&p.path).with_context(|| format!("load {:?}", p.path))?;
    let scaled: RgbaImage = imageops::resize(&src, p.w, p.h, imageops::FilterType::Lanczos3);

    let rotated = match p.rotation {
        Some(angle) => effects::rotate(&scaled, angle),
        None => scaled,
    };

    let mut tile = rotated;
    if opts.corner_radius > 0 {
        effects::apply_rounded_corners(&mut tile, opts.corner_radius);
    }

    composite_over_white(canvas, &tile, p.x, p.y);
    Ok(())
}

/// Alpha-blend an RGBA tile over the (already white) RGB canvas.
fn composite_over_white(canvas: &mut RgbImage, tile: &RgbaImage, off_x: u32, off_y: u32) {
    let (tw, th) = tile.dimensions();
    let (cw, ch) = canvas.dimensions();
    for ty in 0..th {
        let cy = off_y + ty;
        if cy >= ch {
            break;
        }
        for tx in 0..tw {
            let cx = off_x + tx;
            if cx >= cw {
                break;
            }
            let px = tile.get_pixel(tx, ty);
            let a = px[3];
            if a == 0 {
                continue; // fully transparent — keep white background
            }
            if a == 255 {
                canvas.put_pixel(cx, cy, Rgb([px[0], px[1], px[2]]));
                continue;
            }
            let af = a as f32 / 255.0;
            let bg = canvas.get_pixel(cx, cy);
            canvas.put_pixel(
                cx,
                cy,
                Rgb([
                    blend(px[0], bg[0], af),
                    blend(px[1], bg[1], af),
                    blend(px[2], bg[2], af),
                ]),
            );
        }
    }
}

fn blend(src: u8, dst: u8, alpha: f32) -> u8 {
    (src as f32 * alpha + dst as f32 * (1.0 - alpha)).round() as u8
}

/// Load image as RGB8 without applying EXIF orientation (see `load_raw` contract).
fn load_raw(path: &Path) -> Result<RgbImage> {
    Ok(ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?
        .into_rgb8())
}

/// Load image as RGBA8 without applying EXIF orientation.
fn load_rgba(path: &Path) -> Result<RgbaImage> {
    Ok(ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?
        .into_rgba8())
}

fn render_bar(n: usize) -> ProgressBar {
    let pb = ProgressBar::new(n as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} Rendering [{bar:30.green/dim}] {pos}/{len}  {msg}",
        )
        .unwrap()
        .progress_chars("=> "),
    );
    pb
}
