use anyhow::Result;
use clap::Parser;
use collage_maker::{args::Args, collect, effects::EffectOptions, layout, output, render};
use rand::SeedableRng;
use rand::rngs::StdRng;

fn main() -> Result<()> {
    let args = Args::parse();

    let (canvas_w, canvas_h) = args.canvas_px();

    let paths = collect::gather(&args.inputs)?;
    eprintln!("{} image(s) found", paths.len());

    let metas = collect::probe_all(&paths);
    anyhow::ensure!(!metas.is_empty(), "no readable images");

    eprintln!("Canvas: {}×{} px", canvas_w, canvas_h);
    let mut placements = layout::compute(&metas, canvas_w, canvas_h, args.gap);

    // Assign random rotation angles after layout so layout.rs stays rand-free.
    if args.max_rotation > 0.0 {
        use rand::RngExt;
        let mut rng = StdRng::seed_from_u64(42);
        let max_rad = args.max_rotation.to_radians();
        for p in &mut placements {
            p.rotation = Some(rng.random_range(-max_rad..=max_rad));
        }
    }

    let opts = EffectOptions {
        corner_radius: args.corner_radius,
        border_noise: args.border_noise,
    };

    let collage = render::draw(&placements, canvas_w, canvas_h, &opts)?;

    eprintln!("Saving {:?}…", args.output);
    output::save(collage, &args.output, args.dpi)?;

    open_with_viewer(&args.output);

    eprintln!("Done.");
    Ok(())
}

fn open_with_viewer(path: &std::path::Path) {
    let viewer = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };

    match std::process::Command::new(viewer).arg(path).spawn() {
        Ok(_) => eprintln!("Opened in system viewer."),
        Err(e) => eprintln!("Could not open viewer ({viewer}): {e}"),
    }
}
