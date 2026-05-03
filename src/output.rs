use anyhow::{Context, Result};
use image::{DynamicImage, RgbImage, codecs::jpeg::JpegEncoder};
use std::{io::BufWriter, path::Path};

/// Save `img` to `path`; format is inferred from the file extension.
/// Supported extensions: jpg/jpeg → JPEG 90%, png → PNG, pdf → PDF.
/// Unknown extensions fall back to JPEG.
pub fn save(img: RgbImage, path: &Path, dpi: f64) -> Result<()> {
    match extension(path) {
        "pdf" => save_pdf(img, path, dpi),
        "png" => save_png(img, path),
        _ => save_jpeg(img, path),
    }
}

fn extension(path: &Path) -> &str {
    path.extension().and_then(|e| e.to_str()).unwrap_or("jpg")
}

fn save_jpeg(img: RgbImage, path: &Path) -> Result<()> {
    let mut w =
        BufWriter::new(std::fs::File::create(path).with_context(|| format!("create {:?}", path))?);
    let enc = JpegEncoder::new_with_quality(&mut w, 90);
    DynamicImage::ImageRgb8(img)
        .write_with_encoder(enc)
        .context("encode JPEG")
}

fn save_png(img: RgbImage, path: &Path) -> Result<()> {
    img.save(path)
        .with_context(|| format!("save PNG {:?}", path))
}

fn save_pdf(img: RgbImage, path: &Path, dpi: f64) -> Result<()> {
    use printpdf::{Image, ImageTransform, Mm, PdfDocument};

    let (pw, ph) = img.dimensions();
    let w_mm = (pw as f64 / dpi * 25.4) as f32;
    let h_mm = (ph as f64 / dpi * 25.4) as f32;

    let (doc, p1, l1) = PdfDocument::new("Collage", Mm(w_mm), Mm(h_mm), "Layer 1");
    let layer = doc.get_page(p1).get_layer(l1);

    // printpdf 0.7 depends on image 0.24 while we use image 0.25.
    // Bridge: move raw bytes (packed RGB, same layout in both versions) into
    // printpdf's bundled copy of the image crate.
    let raw = img.into_raw();
    let pdf_rgb = printpdf::image_crate::RgbImage::from_raw(pw, ph, raw)
        .context("raw image dimensions mismatch")?;
    let pdf_dyn = printpdf::image_crate::DynamicImage::ImageRgb8(pdf_rgb);

    let pdf_img = Image::from_dynamic_image(&pdf_dyn);
    pdf_img.add_to_layer(
        layer,
        ImageTransform {
            dpi: Some(dpi as f32),
            ..Default::default()
        },
    );

    let mut file =
        BufWriter::new(std::fs::File::create(path).with_context(|| format!("create {:?}", path))?);
    doc.save(&mut file).context("write PDF")
}
