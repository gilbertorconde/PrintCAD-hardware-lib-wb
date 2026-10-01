//! Every external thread case at several start angles, to find the angle
//! the kernel's boolean accepts for all of them.
#[path = "../../../../src/diagram.rs"]
pub mod diagram;
#[path = "../../../../src/geom.rs"]
pub mod geom;
#[path = "../../../../src/parts/mod.rs"]
pub mod parts;
#[path = "../../../../src/standards.rs"]
pub mod standards;

use kernel_api::{SolidOp, TessellationSettings};
use parts::rod::{Rod, RodKind};
use parts::screw::{Head, Screw};
use parts::{Defaults, Part};

/// A case: the body without its thread, and the groove's numbers
/// `(open, bottom, pitch, z_top, end, start)`.
struct Case {
    name: String,
    body: Vec<SolidOp>,
    groove: (f64, f64, f64, f64, f64, f64),
}

fn main() {
    let d = Defaults::default();
    let mut cases = Vec::new();
    for size in ["M3", "M4", "M5", "M6", "M8"] {
        let mut rod = Rod::new(RodKind::Threaded, size, &d);
        rod.length = 12.0;
        let (r, p, l) = (rod.d / 2.0, rod.pitch, rod.length);
        cases.push(Case {
            name: format!("rod {size}"),
            body: rod.ops(),
            groove: (r, r - 0.6134 * p, p, l, l - 1.5 * p, 1.5 * p),
        });
    }
    let screws = [
        (Head::SocketCap, "M3", 8.0),
        (Head::SocketCap, "M4", 10.0),
        (Head::SocketCap, "M5", 12.0),
        (Head::SocketCap, "M6", 16.0),
        (Head::SocketCap, "M3", 20.0),
        (Head::Set, "M3", 6.0),
        (Head::Hex, "M3", 8.0),
        (Head::Hex, "M4", 10.0),
        (Head::Button, "M4", 12.0),
        (Head::Countersunk, "M4", 12.0),
    ];
    for (head, size, length) in screws {
        let mut screw = Screw::new(head, size, &d);
        screw.length = length;
        screw.socket = false;
        let (r, p) = (screw.d / 2.0, screw.pitch);
        let shank = if head == Head::Countersunk { length - screw.k } else { length };
        let b = if matches!(head, Head::Set | Head::Hex) { shank } else { (2.0 * screw.d + 6.0).min(shank) };
        let lead_in = b < shank - 1e-6;
        let start = if lead_in { -p } else { 1.5 * p };
        cases.push(Case {
            name: format!("screw {} {size} × {length}", head.tool()),
            body: screw.ops(),
            groove: (r, r - 0.6134 * p, p, -(shank - b), b - 1.5 * p, start),
        });
    }
    let angles = [0.0, 17.0, 37.0, 53.0, 71.0, 101.0, 137.0, 203.0];
    let mut kernel = kernel_ogeom::OgeomKernel::new();
    print!("{:<36}", "case");
    for a in angles {
        print!(" {a:>5}°");
    }
    println!();
    let mut passes = vec![0usize; angles.len()];
    for case in &cases {
        print!("{:<36}", case.name);
        for (i, angle) in angles.iter().enumerate() {
            let (open, bottom, pitch, z_top, end, start) = case.groove;
            let mut ops = case.body.clone();
            ops.push(geom::thread_groove_from(open, bottom, pitch, z_top, end, start, 0.5 * pitch, *angle, false));
            let ok = kernel.execute_solid_chain(&ops, &TessellationSettings::default()).is_ok();
            if ok {
                passes[i] += 1;
            }
            print!(" {:>6}", if ok { "ok" } else { "FAIL" });
        }
        println!();
    }
    print!("{:<36}", "passes");
    for p in passes {
        print!(" {p:>6}");
    }
    println!(" of {}", cases.len());
}
