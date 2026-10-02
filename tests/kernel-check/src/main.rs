//! Every part, built by the kernel: its ops run through `kernel_ogeom`,
//! and the solid that comes out is checked to be closed, to have volume,
//! and to span its axis. Prints a table and exits 1 if any part fails.

use std::time::Instant;

#[path = "../../../src/diagram.rs"]
pub mod diagram;
#[path = "../../../src/geom.rs"]
pub mod geom;
#[path = "../../../src/parts/mod.rs"]
pub mod parts;
#[path = "../../../src/standards.rs"]
pub mod standards;

mod cases;

use cases::cases;
use kernel_api::TessellationSettings;

fn main() {
    let mut kernel = kernel_ogeom::OgeomKernel::new();
    let detail = TessellationSettings::default();
    let cases = cases();
    let mut failed = 0;
    println!(
        "{:<48} {:>4} {:>12} {:>10} {:>8}  outcome",
        "part", "ops", "volume mm³", "axis span", "ms"
    );
    for case in &cases {
        if let Some(problem) = case.part.problem() {
            println!(
                "{:<48} {:>4} {:>12} {:>10} {:>8}  refused: {problem}",
                case.name, "-", "-", "-", "-"
            );
            failed += 1;
            continue;
        }
        let ops = case.part.ops();
        let (from, to) = case.part.axis();
        let span =
            ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2) + (to[2] - from[2]).powi(2))
                .sqrt();
        let start = Instant::now();
        let built = kernel.execute_solid_chain(&ops, &detail);
        let ms = start.elapsed().as_millis();
        match built {
            Err(e) => {
                println!(
                    "{:<48} {:>4} {:>12} {:>10.2} {:>8}  FAILED at op {}: {}",
                    case.name,
                    ops.len(),
                    "-",
                    span,
                    ms,
                    e.op_index,
                    e.message
                );
                failed += 1;
            }
            Ok(result) => {
                let volume = kernel
                    .physical_properties(&result.brep_blob)
                    .ok()
                    .and_then(|p| p.volume_mm3);
                let bounds = result.bounds_mm;
                // The solid spans its axis: its bounds along the axis'
                // direction reach as far as the axis does.
                let axis_dir = [
                    (to[0] - from[0]) / span,
                    (to[1] - from[1]) / span,
                    (to[2] - from[2]) / span,
                ];
                let spanned = bounds.map(|(lo, hi)| {
                    (0..3)
                        .map(|k| (hi[k] - lo[k]) as f64 * axis_dir[k].abs())
                        .sum::<f64>()
                });
                let mut problems = Vec::new();
                match volume {
                    Some(v) if v > 0.0 => {}
                    Some(v) => problems.push(format!("volume {v}")),
                    None => problems.push("not a closed solid".into()),
                }
                if let Some(s) = spanned
                    && (s - span).abs() > 0.05 * span + 0.1
                {
                    problems.push(format!("spans {s:.2}, axis {span:.2}"));
                }
                if result.mesh.indices.is_empty() {
                    problems.push("no mesh".into());
                }
                let outcome = if problems.is_empty() {
                    "ok".to_string()
                } else {
                    failed += 1;
                    format!("FAILED: {}", problems.join("; "))
                };
                println!(
                    "{:<48} {:>4} {:>12.2} {:>10.2} {:>8}  {outcome}",
                    case.name,
                    ops.len(),
                    volume.unwrap_or(0.0),
                    span,
                    ms
                );
            }
        }
    }
    println!();
    println!("{} parts, {} failed", cases.len(), failed);
    if failed > 0 {
        std::process::exit(1);
    }
}
