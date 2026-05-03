use anyhow::Result;
use clap::Parser;
use collage_maker::{args::Args, collect, layout, output, render};

fn main() -> Result<()> {
    let args = Args::parse();

    let (canvas_w, canvas_h) = args.canvas_px();

    let paths = collect::gather(&args.inputs)?;
    eprintln!("{} image(s) found", paths.len());

    let metas = collect::probe_all(&paths);
    anyhow::ensure!(!metas.is_empty(), "no readable images");

    eprintln!("Canvas: {}×{} px", canvas_w, canvas_h);
    let placements = layout::compute(&metas, canvas_w, canvas_h, args.gap);

    eprintln!("Rendering…");
    let collage = render::draw(&placements, canvas_w, canvas_h)?;

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
        Ok(_)  => eprintln!("Opened in system viewer."),
        Err(e) => eprintln!("Could not open viewer ({viewer}): {e}"),
    }
}
