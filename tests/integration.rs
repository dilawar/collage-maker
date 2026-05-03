use std::path::{Path, PathBuf};

fn test_images_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/images")
}

fn image_paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(|n| test_images_dir().join(n)).collect()
}

fn all_test_images() -> Vec<PathBuf> {
    let dir = test_images_dir();
    std::fs::read_dir(&dir)
        .expect("tests/images not found")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("jpg" | "png")))
        .collect()
}

// ── collect ──────────────────────────────────────────────────────────────────

mod collect_tests {
    use super::*;
    use collage_maker::collect;

    #[test]
    fn gather_directory_finds_all_images() {
        let dir = test_images_dir();
        let paths = collect::gather(&[dir]).unwrap();
        assert_eq!(
            paths.len(),
            8,
            "expected 8 test images, got {}",
            paths.len()
        );
    }

    #[test]
    fn gather_explicit_files() {
        let selected = image_paths(&["landscape_a.jpg", "portrait_a.jpg", "square_a.jpg"]);
        let paths = collect::gather(&selected).unwrap();
        assert_eq!(paths.len(), 3);
    }

    #[test]
    fn gather_mixed_files_and_dirs() {
        let dir = test_images_dir();
        let single = test_images_dir().join("wide.jpg");
        // dir + explicit file from same dir → should not de-dup (gather is faithful)
        let paths = collect::gather(&[dir, single]).unwrap();
        assert!(paths.len() >= 8);
    }

    #[test]
    fn gather_empty_inputs_errors() {
        let result = collect::gather(&[PathBuf::from("/nonexistent/path/xyz")]);
        assert!(result.is_err(), "should error when no images found");
    }

    #[test]
    fn probe_all_returns_correct_aspects() {
        let paths = image_paths(&["landscape_a.jpg", "portrait_a.jpg", "square_a.jpg"]);
        let metas = collect::probe_all(&paths);
        assert_eq!(metas.len(), 3);

        // landscape_a.jpg is 800×600 → aspect ≈ 1.333
        let land = metas
            .iter()
            .find(|m| m.path.ends_with("landscape_a.jpg"))
            .unwrap();
        assert!(
            (land.aspect - 800.0 / 600.0).abs() < 0.01,
            "unexpected aspect {}",
            land.aspect
        );

        // portrait_a.jpg is 600×900 → aspect ≈ 0.667
        let port = metas
            .iter()
            .find(|m| m.path.ends_with("portrait_a.jpg"))
            .unwrap();
        assert!(
            (port.aspect - 600.0 / 900.0).abs() < 0.01,
            "unexpected aspect {}",
            port.aspect
        );

        // square_a.jpg is 600×600 → aspect = 1.0
        let sq = metas
            .iter()
            .find(|m| m.path.ends_with("square_a.jpg"))
            .unwrap();
        assert!(
            (sq.aspect - 1.0).abs() < 0.01,
            "unexpected aspect {}",
            sq.aspect
        );
    }

    #[test]
    fn probe_all_skips_bad_file() {
        let bad = PathBuf::from("/tmp/not_an_image_xyz.jpg");
        let good = test_images_dir().join("square_a.jpg");
        let metas = collect::probe_all(&[bad, good]);
        assert_eq!(metas.len(), 1, "bad file should be skipped silently");
    }
}

// ── layout ───────────────────────────────────────────────────────────────────

mod layout_tests {
    use super::*;
    use collage_maker::{collect, layout};

    fn metas_for(names: &[&str]) -> Vec<collect::Meta> {
        let paths = image_paths(names);
        collect::probe_all(&paths)
    }

    #[test]
    fn single_image_fills_canvas_width() {
        let metas = metas_for(&["landscape_a.jpg"]);
        let placements = layout::compute(&metas, 1240, 1754, 0);
        assert_eq!(placements.len(), 1);
        assert_eq!(placements[0].x, 0);
        assert_eq!(placements[0].w, 1240, "single image must fill canvas width");
    }

    #[test]
    fn placements_stay_within_canvas() {
        let paths = all_test_images();
        let metas = collect::probe_all(&paths);
        let (cw, ch) = (1240u32, 1754u32);
        let placements = layout::compute(&metas, cw, ch, 4);

        for p in &placements {
            assert!(
                p.x + p.w <= cw + 2,
                "image overflows canvas right: x={} w={} cw={}",
                p.x,
                p.w,
                cw
            );
            assert!(
                p.y + p.h <= ch + 2,
                "image overflows canvas bottom: y={} h={} ch={}",
                p.y,
                p.h,
                ch
            );
        }
    }

    #[test]
    fn all_images_are_placed() {
        let paths = all_test_images();
        let metas = collect::probe_all(&paths);
        let n = metas.len();
        let placements = layout::compute(&metas, 1240, 1754, 4);
        assert_eq!(placements.len(), n, "every image must get a placement");
    }

    #[test]
    fn no_image_is_trimmed() {
        // Aspect ratio must be preserved (w/h ≈ original aspect, within rounding)
        let names = &[
            "landscape_a.jpg",
            "portrait_a.jpg",
            "square_a.jpg",
            "wide.jpg",
            "tall.jpg",
        ];
        let metas = metas_for(names);
        let placements = layout::compute(&metas, 1240, 1754, 0);

        for (meta, p) in metas.iter().zip(placements.iter()) {
            if p.h == 0 {
                continue;
            }
            let placed_aspect = p.w as f64 / p.h as f64;
            let diff = (placed_aspect - meta.aspect).abs() / meta.aspect;
            assert!(
                diff < 0.02,
                "aspect mismatch for {:?}: original={:.3} placed={:.3}",
                meta.path.file_name().unwrap(),
                meta.aspect,
                placed_aspect
            );
        }
    }

    #[test]
    fn empty_input_returns_empty_placements() {
        let placements = layout::compute(&[], 1240, 1754, 4);
        assert!(placements.is_empty());
    }

    #[test]
    fn layout_uses_gap() {
        let metas = metas_for(&["landscape_a.jpg", "landscape_b.jpg"]);
        let with_gap = layout::compute(&metas, 1240, 1754, 20);
        let no_gap = layout::compute(&metas, 1240, 1754, 0);

        // When images end up in the same row the gap shifts the second image right
        if with_gap.len() == 2 && with_gap[0].y == with_gap[1].y {
            assert!(
                with_gap[1].x > no_gap[1].x,
                "gap should push second image further right"
            );
        }
    }

    #[test]
    fn layout_square_canvas() {
        let paths = all_test_images();
        let metas = collect::probe_all(&paths);
        let placements = layout::compute(&metas, 1000, 1000, 4);
        assert_eq!(placements.len(), metas.len());
        for p in &placements {
            assert!(p.x + p.w <= 1002);
            assert!(p.y + p.h <= 1002);
        }
    }
}

// ── render ───────────────────────────────────────────────────────────────────

mod render_tests {
    use super::*;
    use collage_maker::{collect, effects::EffectOptions, layout, render};

    #[test]
    fn render_correct_canvas_size() {
        let paths = image_paths(&["landscape_a.jpg", "portrait_a.jpg"]);
        let metas = collect::probe_all(&paths);
        let placements = layout::compute(&metas, 1240, 1754, 4);
        let canvas = render::draw(&placements, 1240, 1754, &EffectOptions::none()).unwrap();
        assert_eq!(canvas.width(), 1240);
        assert_eq!(canvas.height(), 1754);
    }

    #[test]
    fn render_is_not_all_white() {
        let paths = image_paths(&["square_a.jpg"]);
        let metas = collect::probe_all(&paths);
        let placements = layout::compute(&metas, 600, 600, 0);
        let canvas = render::draw(&placements, 600, 600, &EffectOptions::none()).unwrap();

        let non_white = canvas
            .pixels()
            .filter(|p| p[0] != 255 || p[1] != 255 || p[2] != 255)
            .count();
        assert!(
            non_white > 1000,
            "canvas appears blank — image not rendered"
        );
    }

    #[test]
    fn render_many_images_succeeds() {
        let paths = all_test_images();
        let metas = collect::probe_all(&paths);
        let placements = layout::compute(&metas, 1240, 1754, 4);
        let canvas = render::draw(&placements, 1240, 1754, &EffectOptions::none()).unwrap();
        assert_eq!((canvas.width(), canvas.height()), (1240, 1754));
    }
}

// ── output ───────────────────────────────────────────────────────────────────

mod output_tests {
    use super::*;
    use collage_maker::{collect, effects::EffectOptions, layout, output, render};
    use std::io::Read;

    fn make_collage(cw: u32, ch: u32) -> image::RgbImage {
        let paths = image_paths(&["landscape_a.jpg", "portrait_a.jpg", "square_a.jpg"]);
        let metas = collect::probe_all(&paths);
        let placements = layout::compute(&metas, cw, ch, 4);
        render::draw(&placements, cw, ch, &EffectOptions::none()).unwrap()
    }

    #[test]
    fn save_jpeg() {
        let tmp = tempfile("/tmp/collage_test_out.jpg");
        output::save(make_collage(800, 600), &tmp, 150.0).unwrap();
        assert!(tmp.exists());
        assert!(tmp.metadata().unwrap().len() > 1000);
        // JPEG magic bytes: FF D8 FF
        let hdr = read_header(&tmp, 3);
        assert_eq!(&hdr, &[0xFF, 0xD8, 0xFF], "not a valid JPEG");
    }

    #[test]
    fn save_png() {
        let tmp = tempfile("/tmp/collage_test_out.png");
        output::save(make_collage(400, 600), &tmp, 150.0).unwrap();
        assert!(tmp.exists());
        let hdr = read_header(&tmp, 4);
        assert_eq!(&hdr, &[0x89, 0x50, 0x4E, 0x47], "not a valid PNG");
    }

    #[test]
    fn save_pdf() {
        let tmp = tempfile("/tmp/collage_test_out.pdf");
        output::save(make_collage(1240, 1754), &tmp, 150.0).unwrap();
        assert!(tmp.exists());
        assert!(tmp.metadata().unwrap().len() > 1000);
        // PDF magic: %PDF
        let hdr = read_header(&tmp, 4);
        assert_eq!(&hdr, b"%PDF", "not a valid PDF");
    }

    #[test]
    fn unknown_extension_falls_back_to_jpeg() {
        let tmp = tempfile("/tmp/collage_test_out.bmp"); // unsupported → JPEG
        output::save(make_collage(400, 300), &tmp, 150.0).unwrap();
        let hdr = read_header(&tmp, 3);
        assert_eq!(&hdr, &[0xFF, 0xD8, 0xFF], "fallback should produce JPEG");
    }

    fn tempfile(path: &str) -> PathBuf {
        PathBuf::from(path)
    }

    fn read_header(path: &Path, n: usize) -> Vec<u8> {
        let mut f = std::fs::File::open(path).unwrap();
        let mut buf = vec![0u8; n];
        f.read_exact(&mut buf).unwrap();
        buf
    }
}
