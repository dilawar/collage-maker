use anyhow::{Context, Result};
use image::ImageReader;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const IMG_EXTS: &[&str] = &["jpg", "jpeg", "png", "bmp", "gif", "tiff", "tif", "webp"];

/// Aspect ratio and path for a single image (no pixel data loaded yet).
pub struct Meta {
    pub path: PathBuf,
    /// width / height
    pub aspect: f64,
}

/// Expand inputs (files and/or directories) into a sorted list of image paths.
pub fn gather(inputs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for input in inputs {
        if input.is_dir() {
            collect_from_dir(input, &mut paths)?;
        } else if input.is_file() {
            paths.push(input.clone());
        } else {
            eprintln!("Warning: {:?} does not exist, skipping", input);
        }
    }
    anyhow::ensure!(!paths.is_empty(), "no images found in the given inputs");
    Ok(paths)
}

fn collect_from_dir(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in WalkDir::new(dir).sort_by_file_name() {
        let entry = entry?;
        if entry.file_type().is_file() && is_image(entry.path()) {
            out.push(entry.into_path());
        }
    }
    Ok(())
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMG_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Read image dimensions without decoding pixel data, skipping files that fail.
pub fn probe_all(paths: &[PathBuf]) -> Vec<Meta> {
    paths
        .iter()
        .filter_map(|p| match probe(p) {
            Ok(m) => Some(m),
            Err(e) => {
                eprintln!("skip {:?}: {e}", p);
                None
            }
        })
        .collect()
}

fn probe(path: &Path) -> Result<Meta> {
    let (w, h) = ImageReader::open(path)
        .with_context(|| format!("open {:?}", path))?
        .with_guessed_format()?
        .into_dimensions()
        .with_context(|| format!("read dimensions {:?}", path))?;
    anyhow::ensure!(w > 0 && h > 0, "zero-size image {:?}", path);
    Ok(Meta {
        path: path.to_owned(),
        aspect: w as f64 / h as f64,
    })
}
