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

use parts::bearing::BearingKind;
use parts::magnet::MagnetShape;
use parts::nut::NutKind;
use parts::rod::RodKind;
use parts::screw::Head;
use parts::washer::WasherKind;
use parts::extrusion::Slots;
use parts::tnut::TNutKind;
use parts::{
    Bearing, Defaults, Extrusion, Hardware, Insert, Magnet, Nut, Rod, Screw, Spring, TNut, Washer,
};
use kernel_api::TessellationSettings;

struct Case {
    name: String,
    part: Hardware,
}

fn cases() -> Vec<Case> {
    let d = Defaults::default();
    let mut out = Vec::new();
    let mut case = |name: &str, part: Hardware| out.push(Case { name: name.into(), part });
    for head in Head::ALL {
        let screw = Screw::new(head, "M4", &d);
        case(&format!("screw {} M4", head.tool()), Hardware::Screw(screw.clone()));
        let mut plain = screw.clone();
        plain.socket = false;
        case(&format!("screw {} M4, no recess", head.tool()), Hardware::Screw(plain));
    }
    let mut threaded = Screw::new(Head::SocketCap, "M3", &d);
    threaded.thread = true;
    threaded.length = 8.0;
    case("screw socket_cap M3 × 8, modelled thread", Hardware::Screw(threaded));
    let mut inch = Screw::new(Head::SocketCap, "M6", &d);
    inch.standard = "ASME B18.3 (inch)".into();
    inch.size = "1/4-20".into();
    inch.refill();
    case("screw socket_cap 1/4-20", Hardware::Screw(inch));
    let mut set = Screw::new(Head::Set, "M3", &d);
    set.thread = true;
    set.length = 6.0;
    case("screw set_screw M3 × 6, modelled thread", Hardware::Screw(set));
    for kind in NutKind::ALL {
        case(&format!("nut {} M4", kind.tool()), Hardware::Nut(Nut::new(kind, "M4", &d)));
    }
    let mut threaded = Nut::new(NutKind::Hex, "M3", &d);
    threaded.thread = true;
    case("nut hex M3, modelled thread", Hardware::Nut(threaded));
    for kind in WasherKind::ALL {
        case(&format!("washer {} M4", kind.tool()), Hardware::Washer(Washer::new(kind, "M4")));
    }
    case("extrusion 2020 × 40", Hardware::Extrusion(Extrusion::new(20, 1, 1, 40.0)));
    case("extrusion 2040 × 40", Hardware::Extrusion(Extrusion::new(20, 1, 2, 40.0)));
    let mut v = Extrusion::new(20, 1, 1, 40.0);
    v.v_slot = true;
    case("extrusion 2020 × 40, V-slot", Hardware::Extrusion(v));
    let mut x = Extrusion::new(40, 1, 1, 60.0);
    x.along = "X".into();
    case("extrusion 4040 × 60 along X", Hardware::Extrusion(x));
    case("extrusion 3060 × 30", Hardware::Extrusion(Extrusion::new(30, 1, 2, 30.0)));
    for slots in [Slots::Three, Slots::Adjacent, Slots::Opposite, Slots::One] {
        let mut e = Extrusion::new(20, 1, 1, 30.0);
        e.slots = slots;
        case(&format!("extrusion 2020 × 30, {:?}", slots), Hardware::Extrusion(e));
    }
    let mut f = Extrusion::new(20, 1, 2, 30.0);
    f.slots = Slots::Three;
    case("extrusion 2040 × 30, Three", Hardware::Extrusion(f));
    case("extrusion 3030 × 30", Hardware::Extrusion(Extrusion::new(30, 1, 1, 30.0)));
    case("extrusion 4040 × 30", Hardware::Extrusion(Extrusion::new(40, 1, 1, 30.0)));
    case("extrusion 2080 × 30", Hardware::Extrusion(Extrusion::new(20, 1, 4, 30.0)));
    for series in [20, 30, 40] {
        for kind in TNutKind::ALL {
            case(
                &format!("tnut {:?} {series}", kind),
                Hardware::TNut(TNut::new(kind, series, "M5", &d)),
            );
        }
    }
    let mut threaded = TNut::new(TNutKind::DropIn, 20, "M5", &d);
    threaded.thread = true;
    case("tnut DropIn 20 M5, modelled thread", Hardware::TNut(threaded));
    case("insert M3", Hardware::Insert(Insert::new("M3", &d)));
    let mut insert = Insert::new("M4", &d);
    insert.thread = true;
    case("insert M4, modelled thread", Hardware::Insert(insert));
    case("bearing 608", Hardware::Bearing(Bearing::new(BearingKind::Ball, "608")));
    case("bearing MR85", Hardware::Bearing(Bearing::new(BearingKind::Ball, "MR85")));
    case("bearing LM8UU", Hardware::Bearing(Bearing::new(BearingKind::Linear, "LM8UU")));
    for shape in MagnetShape::ALL {
        case(&format!("magnet {}", shape.name()), Hardware::Magnet(Magnet::new(shape)));
    }
    for kind in RodKind::ALL {
        let mut rod = Rod::new(kind, "Ø8", &d);
        rod.length = 30.0;
        case(&format!("rod {} × 30", kind.tool()), Hardware::Rod(rod));
    }
    let mut rod = Rod::new(RodKind::Threaded, "M6", &d);
    rod.length = 12.0;
    rod.thread = true;
    case("rod threaded_rod M6 × 12, modelled thread", Hardware::Rod(rod));
    for (size, length) in [("M3", 12.0), ("M4", 12.0), ("M5", 12.0), ("M8", 12.0)] {
        let mut rod = Rod::new(RodKind::Threaded, size, &d);
        rod.length = length;
        rod.thread = true;
        case(&format!("rod threaded_rod {size} × {length}, modelled thread"), Hardware::Rod(rod));
    }
    for (size, length) in [("M4", 10.0), ("M5", 12.0), ("M6", 16.0)] {
        let mut screw = Screw::new(Head::SocketCap, size, &d);
        screw.length = length;
        screw.thread = true;
        case(&format!("screw socket_cap {size} × {length}, modelled thread"), Hardware::Screw(screw));
    }
    for (size, length) in [("M3", 8.0), ("M4", 10.0)] {
        let mut screw = Screw::new(Head::SocketCap, size, &d);
        screw.length = length;
        screw.thread = true;
        screw.socket = false;
        case(&format!("screw socket_cap {size} × {length}, modelled thread, no recess"), Hardware::Screw(screw));
        let mut hex = Screw::new(Head::Hex, size, &d);
        hex.length = length;
        hex.thread = true;
        case(&format!("screw hex_bolt {size} × {length}, modelled thread"), Hardware::Screw(hex));
    }
    let mut long = Screw::new(Head::SocketCap, "M3", &d);
    long.length = 20.0;
    long.thread = true;
    case("screw socket_cap M3 × 20, modelled thread, lead-in", Hardware::Screw(long));
    case("spring Ø10 × 25", Hardware::Spring(Spring::default()));
    out
}

fn main() {
    let mut kernel = kernel_ogeom::OgeomKernel::new();
    let detail = TessellationSettings::default();
    let cases = cases();
    let mut failed = 0;
    println!("{:<48} {:>4} {:>12} {:>10} {:>8}  outcome", "part", "ops", "volume mm³", "axis span", "ms");
    for case in &cases {
        if let Some(problem) = case.part.problem() {
            println!("{:<48} {:>4} {:>12} {:>10} {:>8}  refused: {problem}", case.name, "-", "-", "-", "-");
            failed += 1;
            continue;
        }
        let ops = case.part.ops();
        let (from, to) = case.part.axis();
        let span = ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2) + (to[2] - from[2]).powi(2)).sqrt();
        let start = Instant::now();
        let built = kernel.execute_solid_chain(&ops, &detail);
        let ms = start.elapsed().as_millis();
        match built {
            Err(e) => {
                println!("{:<48} {:>4} {:>12} {:>10.2} {:>8}  FAILED at op {}: {}", case.name, ops.len(), "-", span, ms, e.op_index, e.message);
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
                let axis_dir = [(to[0] - from[0]) / span, (to[1] - from[1]) / span, (to[2] - from[2]) / span];
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
