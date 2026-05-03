use crate::collect::Meta;
use indicatif::{ProgressBar, ProgressStyle};

/// Pixel coordinates and size for one image on the canvas.
pub struct Placement {
    pub path: std::path::PathBuf,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Entry point: given image metadata and canvas dimensions, return placements.
///
/// Strategy: try row counts 1..=min(n,20), pick the one whose total height is
/// closest to the canvas height (penalising overflow more than dead space).
/// Each row is scaled so its images fill the full canvas width exactly.
/// If the chosen layout still overflows, all rows are scaled down uniformly.
pub fn compute(metas: &[Meta], canvas_w: u32, canvas_h: u32, gap: u32) -> Vec<Placement> {
    if metas.is_empty() {
        return vec![];
    }

    let cw = canvas_w as f64;
    let ch = canvas_h as f64;
    let g = gap as f64;

    let best_rows = best_partition(metas, cw, ch, g);
    place(metas, &best_rows, cw, ch, g)
}

fn layout_spinner(max_k: usize) -> ProgressBar {
    let pb = ProgressBar::new(max_k as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.cyan} Computing layout [{bar:30.cyan/dim}] {pos}/{len} row configs",
        )
        .unwrap()
        .progress_chars("=> "),
    );
    pb
}

// ── partition ────────────────────────────────────────────────────────────────

/// Try every candidate row count and return the partition with the best score.
fn best_partition(metas: &[Meta], cw: f64, ch: f64, g: f64) -> Vec<Vec<usize>> {
    let max_k = metas.len().min(20);
    let pb = layout_spinner(max_k);
    let result = (1..=max_k)
        .map(|k| {
            let rows = optimal_partition(metas, k);
            pb.inc(1);
            rows
        })
        .min_by(|a, b| {
            let sa = score(a, metas, cw, ch, g);
            let sb = score(b, metas, cw, ch, g);
            sa.partial_cmp(&sb).unwrap()
        })
        .unwrap_or_default();
    pb.finish_and_clear();
    result
}

/// Lower is better. Overflow (total > canvas) is penalised 2×.
fn score(rows: &[Vec<usize>], metas: &[Meta], cw: f64, ch: f64, g: f64) -> f64 {
    let th = total_height(rows, metas, cw, g);
    if th <= ch {
        ch - th          // wasted vertical space
    } else {
        (th - ch) * 2.0  // overflow penalty
    }
}

/// DP: partition `n` images into exactly `k` contiguous groups minimising the
/// maximum per-group aspect-ratio sum (drives rows toward equal height).
fn optimal_partition(metas: &[Meta], k: usize) -> Vec<Vec<usize>> {
    let n = metas.len();
    let k = k.min(n);

    if k == 1 {
        return vec![(0..n).collect()];
    }
    if k == n {
        return (0..n).map(|i| vec![i]).collect();
    }

    let prefix = aspect_prefix(metas);
    let (_dp, spl) = partition_dp(&prefix, n, k);
    reconstruct(&spl, n, k)
}

fn aspect_prefix(metas: &[Meta]) -> Vec<f64> {
    let mut p = vec![0.0f64; metas.len() + 1];
    for (i, m) in metas.iter().enumerate() {
        p[i + 1] = p[i] + m.aspect;
    }
    p
}

/// Returns (dp table, split-point table).
/// dp[i][j] = min cost to partition the first i images into j groups,
/// where cost = max group aspect-ratio sum seen so far.
fn partition_dp(prefix: &[f64], n: usize, k: usize) -> (Vec<Vec<f64>>, Vec<Vec<usize>>) {
    let inf = f64::MAX / 2.0;
    let mut dp = vec![vec![inf; k + 1]; n + 1];
    let mut spl = vec![vec![0usize; k + 1]; n + 1];
    dp[0][0] = 0.0;

    for j in 1..=k {
        for i in j..=n {
            for m in (j - 1)..i {
                if dp[m][j - 1] >= inf {
                    continue;
                }
                let group_sum = prefix[i] - prefix[m];
                let cost = dp[m][j - 1].max(group_sum);
                if cost < dp[i][j] {
                    dp[i][j] = cost;
                    spl[i][j] = m;
                }
            }
        }
    }
    (dp, spl)
}

fn reconstruct(spl: &[Vec<usize>], n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut rows = Vec::with_capacity(k);
    let mut i = n;
    let mut j = k;
    while j > 0 {
        let m = spl[i][j];
        rows.push((m..i).collect());
        i = m;
        j -= 1;
    }
    rows.reverse();
    rows
}

// ── geometry ─────────────────────────────────────────────────────────────────

/// Total canvas height consumed by `rows` when each row fills `cw`.
fn total_height(rows: &[Vec<usize>], metas: &[Meta], cw: f64, g: f64) -> f64 {
    let k = rows.len();
    rows.iter()
        .enumerate()
        .map(|(ri, row)| {
            let rh = row_height(row, metas, cw, g);
            rh + if ri + 1 < k { g } else { 0.0 }
        })
        .sum()
}

/// Height of a single row when scaled to fill `cw` exactly.
fn row_height(row: &[usize], metas: &[Meta], cw: f64, g: f64) -> f64 {
    let aspect_sum: f64 = row.iter().map(|&i| metas[i].aspect).sum();
    let h_gaps = (row.len().saturating_sub(1)) as f64 * g;
    (cw - h_gaps) / aspect_sum
}

// ── placement ────────────────────────────────────────────────────────────────

/// Convert row assignments into pixel placements.
fn place(metas: &[Meta], rows: &[Vec<usize>], cw: f64, ch: f64, g: f64) -> Vec<Placement> {
    let th = total_height(rows, metas, cw, g);
    let scale = uniform_scale(th, ch);
    let y_start = vertical_offset(th, ch, scale);

    let k = rows.len();
    let mut placements = Vec::new();
    let mut y = y_start;

    for (ri, row) in rows.iter().enumerate() {
        let rh = row_height(row, metas, cw, g) * scale;
        place_row(row, metas, rh, g, y, &mut placements);
        y += rh + if ri + 1 < k { g } else { 0.0 };
    }
    placements
}

/// Scale factor to shrink the layout into the canvas (1.0 if it already fits).
fn uniform_scale(total_h: f64, canvas_h: f64) -> f64 {
    if total_h > canvas_h {
        canvas_h / total_h
    } else {
        1.0
    }
}

/// Top margin so the collage is vertically centred when it's shorter than the canvas.
fn vertical_offset(total_h: f64, canvas_h: f64, scale: f64) -> f64 {
    let used = total_h * scale;
    if used < canvas_h {
        (canvas_h - used) / 2.0
    } else {
        0.0
    }
}

/// Emit placements for one row, left-to-right.
fn place_row(
    row: &[usize],
    metas: &[Meta],
    row_h: f64,
    g: f64,
    y: f64,
    out: &mut Vec<Placement>,
) {
    let mut x = 0.0f64;
    for &idx in row {
        let iw = metas[idx].aspect * row_h;
        out.push(Placement {
            path: metas[idx].path.clone(),
            x: x.round() as u32,
            y: y.round() as u32,
            w: iw.round() as u32,
            h: row_h.round() as u32,
        });
        x += iw + g;
    }
}
