use image::{Rgba, RgbaImage};
use rand::{Rng, RngExt};

/// Parameters forwarded from CLI to the rendering step.
pub struct EffectOptions {
    pub corner_radius: u32,
    pub border_noise: u8,
}

impl EffectOptions {
    pub fn none() -> Self {
        Self {
            corner_radius: 0,
            border_noise: 0,
        }
    }

    pub fn active(&self) -> bool {
        self.corner_radius > 0 || self.border_noise > 0
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

/// Apply rounded corners and/or border noise to `img`'s alpha channel in-place.
///
/// The two effects are combined multiplicatively so each is independent:
/// when one is disabled (radius = 0 or noise = 0) it contributes 255 and has no effect.
pub fn apply_alpha_mask(img: &mut RgbaImage, radius: u32, noise_amp: u8, rng: &mut impl Rng) {
    let (w, h) = img.dimensions();
    let radius = radius.min(w / 2).min(h / 2);
    let noise_band = (noise_amp as u32).min(w / 2).min(h / 2);

    for py in 0..h {
        for px in 0..w {
            let a_corner = corner_alpha(px, py, w, h, radius);
            let a_noise = border_noise_alpha(px, py, w, h, noise_band, noise_amp, rng);
            let combined = (a_corner as u16 * a_noise as u16 / 255) as u8;
            img.get_pixel_mut(px, py)[3] = combined;
        }
    }
}

/// Alpha from rounded-corner mask: 0 outside arc, 255 inside, anti-aliased at edge.
fn corner_alpha(px: u32, py: u32, w: u32, h: u32, radius: u32) -> u8 {
    if radius == 0 {
        return 255;
    }
    let r = radius as f32;
    // Determine which corner quadrant this pixel belongs to, if any.
    let (in_corner, cx, cy) = if px < radius && py < radius {
        (true, radius as f32, radius as f32)
    } else if px >= w - radius && py < radius {
        (true, (w - radius) as f32, radius as f32)
    } else if px < radius && py >= h - radius {
        (true, radius as f32, (h - radius) as f32)
    } else if px >= w - radius && py >= h - radius {
        (true, (w - radius) as f32, (h - radius) as f32)
    } else {
        (false, 0.0, 0.0)
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

/// Alpha from border noise: full inside the image, rough/tapered near each edge.
fn border_noise_alpha(
    px: u32,
    py: u32,
    w: u32,
    h: u32,
    noise_band: u32,
    noise_amp: u8,
    rng: &mut impl Rng,
) -> u8 {
    if noise_amp == 0 || noise_band == 0 {
        return 255;
    }
    let dist = edge_distance(px, py, w, h);
    if dist >= noise_band {
        return 255;
    }
    // Noise fades to zero as we approach the edge; inward it reaches full amplitude.
    let t = dist as f32 / noise_band as f32; // 0.0 at edge → 1.0 at inner boundary
    let noise: u8 = rng.random();
    let reduction = ((1.0 - t) * noise as f32) as u8;
    255 - reduction
}

fn edge_distance(px: u32, py: u32, w: u32, h: u32) -> u32 {
    let left = px;
    let right = w - 1 - px;
    let top = py;
    let bottom = h - 1 - py;
    left.min(right).min(top).min(bottom)
}
