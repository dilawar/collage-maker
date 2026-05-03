use clap::Parser;
use std::path::PathBuf;

const A4_W_MM: f64 = 210.0;
const A4_H_MM: f64 = 297.0;

#[derive(Parser, Debug)]
#[command(
    name = "collage",
    about = "Arrange images into a collage (A4 by default)"
)]
pub struct Args {
    /// Input image files or directories (searched recursively)
    #[arg(required = true)]
    pub inputs: Vec<PathBuf>,

    /// Output file — extension sets format: .jpg .png .pdf (default: collage.jpg)
    #[arg(short, long, default_value = "collage.jpg")]
    pub output: PathBuf,

    /// Canvas width in pixels (overrides A4 default)
    #[arg(long)]
    pub width: Option<u32>,

    /// Canvas height in pixels (overrides A4 default)
    #[arg(long)]
    pub height: Option<u32>,

    /// DPI used when deriving A4 pixel dimensions
    #[arg(long, default_value_t = 150.0)]
    pub dpi: f64,

    /// Pixel gap between images
    #[arg(long, default_value_t = 4)]
    pub gap: u32,

    /// Corner arc radius in pixels — 0 disables rounding
    #[arg(long, default_value_t = 0)]
    pub corner_radius: u32,

    /// Border roughness 0–255: noisy alpha falloff at each image edge — 0 disables
    #[arg(long, default_value_t = 0)]
    pub border_noise: u8,

    /// Max random tilt per image in degrees, e.g. 3.0 for ±3° — 0 disables
    #[arg(long, default_value_t = 0.0)]
    pub max_rotation: f32,
}

impl Args {
    pub fn canvas_px(&self) -> (u32, u32) {
        let w = self.width.unwrap_or_else(|| mm_to_px(A4_W_MM, self.dpi));
        let h = self.height.unwrap_or_else(|| mm_to_px(A4_H_MM, self.dpi));
        (w, h)
    }
}

fn mm_to_px(mm: f64, dpi: f64) -> u32 {
    (mm / 25.4 * dpi).round() as u32
}
