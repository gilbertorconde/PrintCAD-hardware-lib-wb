//! Pictures of parts as the kernel builds them: each part's mesh, shaded
//! and drawn to an SVG from a three-quarter view, to look at.
//! `cargo run --release --bin picture` writes /tmp/hardware-<name>.svg.
#[path = "../../../../src/diagram.rs"]
pub mod diagram;
#[path = "../../../../src/geom.rs"]
pub mod geom;
#[path = "../../../../src/parts/mod.rs"]
pub mod parts;
#[path = "../../../../src/standards.rs"]
pub mod standards;

use kernel_api::TessellationSettings;
use parts::nut::{Nut, NutKind};
use parts::rod::{Rod, RodKind};
use parts::screw::{Head, Screw};
use parts::extrusion::{Extrusion, Slots};
use parts::tnut::{TNut, TNutKind};
use parts::{Defaults, Hardware, Insert, Spring};

fn main() {
    let d = Defaults::default();
    let mut screw = Screw::new(Head::SocketCap, "M4", &d);
    screw.length = 10.0;
    screw.thread = true;
    let mut nut = Nut::new(NutKind::Hex, "M4", &d);
    nut.thread = true;
    let mut rod = Rod::new(RodKind::Threaded, "M6", &d);
    rod.length = 10.0;
    rod.thread = true;
    let cases = [
        ("insert-m3", Hardware::Insert(Insert::new("M3", &d))),
        ("screw-m4-thread", Hardware::Screw(screw)),
        ("nut-m4-thread", Hardware::Nut(nut)),
        ("rod-m6-thread", Hardware::Rod(rod)),
        ("spring", Hardware::Spring(Spring::default())),
        ("extrusion-4040", Hardware::Extrusion(Extrusion::new(40, 1, 1, 20.0))),
        ("extrusion-2040", Hardware::Extrusion(Extrusion::new(20, 1, 2, 20.0))),
        ("extrusion-2020-three", {
            let mut e = Extrusion::new(20, 1, 1, 20.0);
            e.slots = Slots::Three;
            Hardware::Extrusion(e)
        }),
        ("tnut-spring-20", Hardware::TNut(TNut::new(TNutKind::SpringBall, 20, "M5", &d))),
        ("tnut-twist-20", Hardware::TNut(TNut::new(TNutKind::Twist, 20, "M4", &d))),
        ("tnut-rollin-30", Hardware::TNut(TNut::new(TNutKind::RollIn, 30, "M5", &d))),
    ];
    let mut kernel = kernel_ogeom::OgeomKernel::new();
    let detail = TessellationSettings::default();
    for (name, part) in cases {
        let built = match kernel.execute_solid_chain(&part.ops(), &detail) {
            Ok(b) => b,
            Err(e) => {
                println!("{name}: FAILED op {}: {}", e.op_index, e.message);
                continue;
            }
        };
        let mesh = built.mesh;
        // A three-quarter view: turn about Z, then tilt.
        let (ca, sa) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
        let (cb, sb) = (60f64.to_radians().cos(), 60f64.to_radians().sin());
        let view = |p: [f32; 3]| -> [f64; 3] {
            let (x, y, z) = (p[0] as f64, p[1] as f64, p[2] as f64);
            let (x1, y1) = (x * ca - y * sa, x * sa + y * ca);
            // Screen x, screen y (up), depth toward the viewer.
            [x1, z * sb + y1 * cb, z * cb - y1 * sb]
        };
        let light = [-0.4f64, 0.6, 0.7];
        let mut tris: Vec<(f64, [[f64; 2]; 3], f64)> = Vec::new();
        for t in mesh.indices.chunks(3) {
            let p: Vec<[f64; 3]> = t.iter().map(|i| view(mesh.positions[*i as usize])).collect();
            let (a, b, c) = (p[0], p[1], p[2]);
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len < 1e-12 || n[2] <= 0.0 {
                continue;
            }
            let n = [n[0] / len, n[1] / len, n[2] / len];
            let shade = (n[0] * light[0] + n[1] * light[1] + n[2] * light[2]).max(0.0) * 0.75 + 0.25;
            let depth = (a[2] + b[2] + c[2]) / 3.0;
            tris.push((depth, [[a[0], a[1]], [b[0], b[1]], [c[0], c[1]]], shade));
        }
        tris.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
        let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for (_, t, _) in &tris {
            for p in t {
                x0 = x0.min(p[0]);
                x1 = x1.max(p[0]);
                y0 = y0.min(p[1]);
                y1 = y1.max(p[1]);
            }
        }
        let scale = 700.0 / (x1 - x0).max(y1 - y0);
        let mut svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" style="background:#1a1f26">"##,
            w = ((x1 - x0) * scale + 40.0) as u32,
            h = ((y1 - y0) * scale + 40.0) as u32
        );
        for (_, t, shade) in &tris {
            let pts: Vec<String> = t
                .iter()
                .map(|p| format!("{:.1},{:.1}", (p[0] - x0) * scale + 20.0, (y1 - p[1]) * scale + 20.0))
                .collect();
            let c = (shade * 200.0) as u8;
            svg.push_str(&format!(
                r##"<polygon points="{}" fill="rgb({},{},{})" stroke="rgb({},{},{})" stroke-width="0.3"/>"##,
                pts.join(" "),
                c, c + 20, c + 45, c, c + 20, c + 45
            ));
        }
        svg.push_str("</svg>");
        let path = format!("/tmp/hardware-{name}.svg");
        std::fs::write(&path, svg).unwrap();
        println!("{name}: {} triangles → {path}", tris.len());
    }
}
