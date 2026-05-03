use image::{Rgba, RgbaImage};

/// Parameters forwarded from CLI to the rendering step.
pub struct EffectOptions {
    pub corner_radius: u32,
}

impl EffectOptions {
    pub fn none() -> Self {
        Self { corner_radius: 0 }
    }

    pub fn active(&self) -> bool {
        self.corner_radius > 0
    }
}

/// Rotate `src` by `angle_rad` around its centre using inverse-mapping bilinear
/// interpolation.  The output has the same dimensions as `src`.  Pixels whose
/// source coordinates fall outside the image bounds become fully transparent.
pub fn rotate(src: &RgbaImage, angle_rad: f32) -> RgbaImage {
    if angle_rad == 0.0 {
        return src.clone();
    }
    let (w, h) = src.dimensions();
    let mut dst = RgbaImage::new(w, h);
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    // Inverse rotation: map each destination pixel back to its source position.
    let cos_a = (-angle_rad).cos();
    let sin_a = (-angle_rad).sin();

    for dy in 0..h {
        for dx in 0..w {
            let fx = cx + (dx as f32 - cx) * cos_a - (dy as f32 - cy) * sin_a;
            let fy = cy + (dx as f32 - cx) * sin_a + (dy as f32 - cy) * cos_a;

            if fx < 0.0 || fy < 0.0 || fx >= (w - 1) as f32 || fy >= (h - 1) as f32 {
                dst.put_pixel(dx, dy, Rgba([0, 0, 0, 0]));
                continue;
            }

            dst.put_pixel(dx, dy, bilinear(src, fx, fy));
        }
    }
    dst
}

fn bilinear(src: &RgbaImage, fx: f32, fy: f32) -> Rgba<u8> {
    let x0 = fx.floor() as u32;
    let y0 = fy.floor() as u32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let tx = fx - fx.floor();
    let ty = fy - fy.floor();

    let p00 = src.get_pixel(x0, y0).0;
    let p10 = src.get_pixel(x1, y0).0;
    let p01 = src.get_pixel(x0, y1).0;
    let p11 = src.get_pixel(x1, y1).0;

    let lerp = |a: u8, b: u8, t: f32| -> u8 { (a as f32 + (b as f32 - a as f32) * t) as u8 };
    Rgba(std::array::from_fn(|i| {
        lerp(lerp(p00[i], p10[i], tx), lerp(p01[i], p11[i], tx), ty)
    }))
}

/// Apply a rounded-corner alpha mask to `img` in-place.
/// Pixels outside the corner arc are set to fully transparent; the arc edge is
/// anti-aliased over a 1-pixel band.
pub fn apply_rounded_corners(img: &mut RgbaImage, radius: u32) {
    if radius == 0 {
        return;
    }
    let (w, h) = img.dimensions();
    let radius = radius.min(w / 2).min(h / 2);

    for py in 0..h {
        for px in 0..w {
            img.get_pixel_mut(px, py)[3] = corner_alpha(px, py, w, h, radius);
        }
    }
}

/// Alpha value for a pixel given rounded-corner geometry.
/// Returns 0 outside the arc, 255 inside, and a linear blend in the 1-px edge band.
fn corner_alpha(px: u32, py: u32, w: u32, h: u32, radius: u32) -> u8 {
    let r = radius as f32;
    let (in_corner, cx, cy) = if px < radius && py < radius {
        (true, r, r)
    } else if px >= w - radius && py < radius {
        (true, (w - radius) as f32, r)
    } else if px < radius && py >= h - radius {
        (true, r, (h - radius) as f32)
    } else if px >= w - radius && py >= h - radius {
        (true, (w - radius) as f32, (h - radius) as f32)
    } else {
        return 255;
    };

    if !in_corner {
        return 255;
    }

    let d = ((px as f32 - cx).powi(2) + (py as f32 - cy).powi(2)).sqrt();
    if d >= r {
        0
    } else if d >= r - 1.0 {
        ((r - d) * 255.0) as u8
    } else {
        255
    }
}
