//! A part put into a hole: which of a body's holes a pick names, the size
//! the hole calls for, and where the part's body goes to sit in it.
//!
//! The holes are the host's (`geom.holes`): each its opening's centre,
//! the way it runs into the material, its diameter and depth, all in the
//! world. A part's own frame runs along Z (see each `axis`), so it sits in
//! a hole once a point of that axis, its seat, is at the opening and the
//! axis lies along the hole's.

use printcad_bench_sdk::api::kernel_api::FaceSurface;
use printcad_bench_sdk::api::{EdgeRef, FaceRef, Pointer};
use printcad_bench_sdk::{Value, json};

use crate::parts::bearing::BearingKind;
use crate::parts::magnet::MagnetShape;
use crate::parts::rod::RodKind;
use crate::parts::{Defaults, Hardware, fmt};
use crate::standards;

/// How near two lengths or a point and a line are to be the same, mm.
const NEAR: f64 = 0.05;

/// A round hole of a body, where the body sits.
#[derive(Debug, Clone, PartialEq)]
pub struct Hole {
    /// The centre of its opening, on its axis.
    pub entry: [f64; 3],
    /// Along its axis, into the material: a unit vector.
    pub direction: [f64; 3],
    pub diameter: f64,
    /// The bore's length from the opening.
    pub depth: f64,
    /// Open at both ends.
    pub through: bool,
}

impl Hole {
    /// One of `geom.holes`'s answer, or a command's `hole` argument.
    pub fn read(v: &Value) -> Option<Hole> {
        let vec3 = |key: &str| -> Option<[f64; 3]> {
            let a = v.get(key)?.as_array()?;
            Some([
                a.first()?.as_f64()?,
                a.get(1)?.as_f64()?,
                a.get(2)?.as_f64()?,
            ])
        };
        let direction = unit(vec3("direction")?)?;
        Some(Hole {
            entry: vec3("entry")?,
            direction,
            diameter: v.get("diameter")?.as_f64()?,
            depth: v.get("depth").and_then(Value::as_f64).unwrap_or(0.0),
            through: v.get("through").and_then(Value::as_bool).unwrap_or(false),
        })
    }

    pub fn to_value(&self) -> Value {
        json!({
            "entry": self.entry,
            "direction": self.direction,
            "diameter": self.diameter,
            "depth": self.depth,
            "through": self.through,
        })
    }

    /// How far `p` stands off the hole's axis.
    fn off_axis(&self, p: [f64; 3]) -> f64 {
        let v = sub(p, self.entry);
        len(sub(v, scale(self.direction, dot(v, self.direction))))
    }

    /// How far along the hole, from its opening, `p` is.
    fn along(&self, p: [f64; 3]) -> f64 {
        dot(sub(p, self.entry), self.direction)
    }

    fn parallel(&self, axis: [f64; 3]) -> bool {
        unit(axis).is_some_and(|a| dot(a, self.direction).abs() > 0.999)
    }

    /// The far end of the bore: a through hole's other opening.
    fn far_end(&self) -> [f64; 3] {
        add(self.entry, scale(self.direction, self.depth))
    }

    /// A through hole run from its other opening.
    fn reversed(&self) -> Hole {
        Hole {
            entry: self.far_end(),
            direction: scale(self.direction, -1.0),
            ..self.clone()
        }
    }

    /// A through hole run from the opening nearer `p`; a blind one as
    /// it is.
    pub fn from_near(&self, p: [f64; 3]) -> Hole {
        if self.through && len(sub(p, self.far_end())) < len(sub(p, self.entry)) {
            self.reversed()
        } else {
            self.clone()
        }
    }

    /// A through hole run along `toward`, from the opening facing away
    /// from it (the one in view, when `toward` is the way the view
    /// looks); a blind one as it is.
    pub fn facing(&self, toward: [f64; 3]) -> Hole {
        if self.through && dot(self.direction, toward) < 0.0 {
            self.reversed()
        } else {
            self.clone()
        }
    }

    /// The same size: what "every hole like this one" gathers.
    pub fn like(&self, other: &Hole) -> bool {
        (self.diameter - other.diameter).abs() < 0.01
    }
}

/// Whether two holes are one: on one axis, from one opening, one size.
pub fn same(a: &Hole, b: &Hole) -> bool {
    len(sub(a.entry, b.entry)) < NEAR
        && dot(a.direction, b.direction) > 0.999
        && (a.diameter - b.diameter).abs() < NEAR
}

/// The holes `pointer`'s picks name, in the order picked, each once: a
/// bore's face, a counterbore's seat or a hole's round edge. A through
/// hole runs from the opening picked: the one whose rim or seat was
/// picked, or for a bore's face the one in view (`toward`, the way the
/// view looks; down when not known).
pub fn picked(holes: &[Hole], pointer: &Pointer, toward: Option<[f64; 3]>) -> Vec<(usize, Hole)> {
    let faces: Vec<&FaceRef> = if pointer.faces.is_empty() {
        pointer.face.iter().collect()
    } else {
        pointer.faces.iter().collect()
    };
    let toward = toward.unwrap_or([0.0, 0.0, -1.0]);
    let mut found: Vec<(usize, Hole)> = Vec::new();
    let named = faces
        .into_iter()
        .map(|f| {
            let i = at_face(holes, f)?;
            let bore = matches!(f.surface, Some(FaceSurface::Cylinder { .. }));
            Some((
                i,
                if bore {
                    holes[i].facing(toward)
                } else {
                    holes[i].from_near(f64s(f.point))
                },
            ))
        })
        .chain(pointer.edges.iter().map(|e| {
            let i = at_edge(holes, e)?;
            Some((i, holes[i].from_near(f64s(e.circle?.center))))
        }));
    for (i, hole) in named.flatten() {
        if !found.iter().any(|(j, _)| *j == i) {
            found.push((i, hole));
        }
    }
    found
}

/// The hole whose bore, or seat round its opening, `face` is.
pub fn at_face(holes: &[Hole], face: &FaceRef) -> Option<usize> {
    let point = f64s(face.point);
    match face.surface? {
        FaceSurface::Cylinder {
            origin,
            axis,
            radius,
        } => {
            let (origin, axis) = (f64s(origin), f64s(axis));
            nearest(holes, |h| {
                let on_axis = h.off_axis(origin) < NEAR && h.parallel(axis);
                let size = (h.diameter - 2.0 * radius as f64).abs() < NEAR;
                // Of two bores in line, the one the point is in.
                let t = h.along(point);
                let outside = (-t).max(t - h.depth).max(0.0);
                (on_axis && size).then_some(outside)
            })
        }
        FaceSurface::Plane { origin, normal } => {
            // A flat ring round an opening: the hole starts on the plane,
            // square to it, and the point is just outside its bore.
            let (origin, normal) = (f64s(origin), f64s(normal));
            nearest(holes, |h| {
                let n = unit(normal)?;
                let on = |p: [f64; 3]| dot(sub(p, origin), n).abs() < NEAR;
                let on_plane = on(h.entry) || (h.through && on(h.far_end()));
                let off = h.off_axis(point);
                let ring = off > h.diameter / 2.0 - NEAR && off < h.diameter;
                (on_plane && h.parallel(normal) && ring).then_some(off)
            })
        }
        _ => None,
    }
}

/// A bore the host does not count as a hole (one opening into a
/// countersink, say), from its picked face and the centres of the round
/// edges that end it: the axis between the outermost two, run into from
/// the end facing `toward` (the way the view looks), or from the top.
pub fn bore(face: &FaceRef, ends: &[[f64; 3]], toward: Option<[f64; 3]>) -> Option<Hole> {
    let Some(FaceSurface::Cylinder {
        origin,
        axis,
        radius,
    }) = face.surface
    else {
        return None;
    };
    let (origin, axis, point) = (f64s(origin), unit(f64s(axis))?, f64s(face.point));
    // A bore's face looks in towards its axis; a shaft's looks out.
    let out = sub(point, origin);
    let out = sub(out, scale(axis, dot(out, axis)));
    if dot(out, f64s(face.normal)) >= 0.0 {
        return None;
    }
    let ats: Vec<f64> = ends
        .iter()
        .filter(|c| {
            let v = sub(**c, origin);
            len(sub(v, scale(axis, dot(v, axis)))) < NEAR
        })
        .map(|c| dot(sub(*c, origin), axis))
        .collect();
    let lo = ats.iter().copied().reduce(f64::min)?;
    let hi = ats.iter().copied().reduce(f64::max)?;
    if hi - lo < NEAR {
        return None;
    }
    let toward = toward.and_then(unit).unwrap_or([0.0, 0.0, -1.0]);
    // Into the material is the way the view looks; along the axis from
    // the end nearer the eye.
    let (direction, from) = if dot(axis, toward) >= 0.0 {
        (axis, lo)
    } else {
        (scale(axis, -1.0), hi)
    };
    Some(Hole {
        entry: [
            origin[0] + axis[0] * from,
            origin[1] + axis[1] * from,
            origin[2] + axis[2] * from,
        ],
        direction,
        diameter: 2.0 * radius as f64,
        depth: hi - lo,
        through: false,
    })
}

/// The hole one of whose round edges `edge` is: its rim, or a seat's.
pub fn at_edge(holes: &[Hole], edge: &EdgeRef) -> Option<usize> {
    let circle = edge.circle?;
    let (center, normal) = (f64s(circle.center), f64s(circle.normal));
    let r = circle.radius as f64;
    nearest(holes, |h| {
        let on_axis = h.off_axis(center) < NEAR && h.parallel(normal);
        // The rim of the bore itself, or a wider seat's round it.
        let size = 2.0 * r > h.diameter - NEAR && 2.0 * r < 2.0 * h.diameter;
        (on_axis && size).then_some(len(sub(center, h.entry)) + (2.0 * r - h.diameter).abs())
    })
}

/// The hole `score` likes best (lowest), of those it scores at all.
fn nearest(holes: &[Hole], score: impl Fn(&Hole) -> Option<f64>) -> Option<usize> {
    holes
        .iter()
        .enumerate()
        .filter_map(|(i, h)| Some((i, score(h)?)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// What a hole asks of a part: the size it is made in.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Fit {
    /// A thread's size (`M3`) or a rod's (`Ø8`).
    pub size: Option<String>,
    /// A bearing's designation.
    pub bearing: Option<&'static str>,
    /// A disc magnet's diameter and height.
    pub magnet: Option<(f64, f64)>,
}

impl Fit {
    pub fn is_empty(&self) -> bool {
        *self == Fit::default()
    }

    /// What it names, for the panel: `M3`, `608`, `Ø6 × 3`.
    pub fn name(&self) -> Option<String> {
        self.size
            .clone()
            .or_else(|| self.bearing.map(String::from))
            .or_else(|| {
                self.magnet
                    .map(|(d, h)| format!("Ø{} × {}", fmt(d), fmt(h)))
            })
    }

    /// Put into a command's arguments what they leave out.
    pub fn into_args(&self, args: &Value) -> Value {
        let mut args = match args {
            Value::Object(_) => args.clone(),
            _ => json!({}),
        };
        let given = |a: &Value, k: &str| a.get(k).is_some();
        if let Some(size) = &self.size
            && !given(&args, "size")
            && !given(&args, "d")
        {
            args["size"] = json!(size);
        }
        if let Some(name) = self.bearing
            && !given(&args, "name")
        {
            args["name"] = json!(name);
        }
        if let Some((d, h)) = self.magnet
            && !given(&args, "d")
        {
            args["d"] = json!(d);
            if !given(&args, "h") {
                args["h"] = json!(h);
            }
        }
        args
    }

    /// `part` remade in this size, as the tool that made it would have.
    pub fn apply(&self, part: Hardware, defaults: &Defaults) -> Hardware {
        use crate::parts::*;
        let mut d = defaults.clone();
        if let Some(size) = &self.size {
            d.size = size.clone();
        }
        let size = d.size.clone();
        match part {
            Hardware::Screw(s) if self.size.is_some() => {
                Hardware::Screw(Screw::new(s.head, &size, &d))
            }
            Hardware::Nut(n) if self.size.is_some() => Hardware::Nut(Nut::new(n.kind, &size, &d)),
            Hardware::Washer(w) if self.size.is_some() => {
                Hardware::Washer(Washer::new(w.kind, &size))
            }
            Hardware::Insert(_) if self.size.is_some() => Hardware::Insert(Insert::new(&size, &d)),
            Hardware::Standoff(s) if self.size.is_some() => {
                let mut new = Standoff::new(s.shape, &size, &d);
                if s.stud > 0.0 {
                    new.stud = 1.0;
                    new.refill();
                }
                Hardware::Standoff(new)
            }
            Hardware::Rod(r) if self.size.is_some() => Hardware::Rod(Rod::new(r.kind, &size, &d)),
            Hardware::Bearing(b) => match self.bearing {
                Some(name) => Hardware::Bearing(Bearing::new(b.kind, name)),
                None => Hardware::Bearing(b),
            },
            Hardware::Magnet(mut m) => {
                if let Some((dia, h)) = self.magnet {
                    m.d = dia;
                    m.h = h;
                }
                Hardware::Magnet(m)
            }
            other => other,
        }
    }
}

/// How a thread's size may answer a hole's diameter.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ThreadHole {
    /// ISO 273's clearance holes, fine to coarse: a screw passes through.
    Clearance,
    /// The tap drill: the thread is cut into it.
    Tap,
    /// A heat-set insert's hole.
    Insert,
    /// The thread's own diameter: a screw driven into plastic.
    Nominal,
}

/// The metric size a hole of `diameter` is made for, trying each way in
/// `ways` in turn and taking the closest size of the first that fits.
fn thread_size(diameter: f64, ways: &[ThreadHole]) -> Option<&'static str> {
    ways.iter().find_map(|way| {
        standards::METRIC_SIZES
            .iter()
            .filter_map(|size| {
                let (d, pitch) = standards::metric(size)?;
                let off = match way {
                    ThreadHole::Clearance => {
                        let h = standards::clearance(d)?;
                        (diameter > h.fine - NEAR && diameter < h.coarse + NEAR)
                            .then_some((diameter - h.medium).abs())
                    }
                    ThreadHole::Tap => {
                        Some((diameter - (d - pitch)).abs()).filter(|off| *off <= 0.15)
                    }
                    ThreadHole::Insert => {
                        let i = standards::insert_for(d)?;
                        Some((diameter - i.hole).abs()).filter(|off| *off <= 0.2)
                    }
                    ThreadHole::Nominal => Some((diameter - d).abs()).filter(|off| *off <= 0.1),
                };
                Some((*size, off?, d))
            })
            // Of sizes alike (an M2.5 and an M3 insert share a hole), the
            // larger.
            .min_by(|a, b| a.1.total_cmp(&b.1).then(b.2.total_cmp(&a.2)))
            .map(|(size, ..)| size)
    })
}

/// A stock diameter a hole of `diameter` takes: a shaft or pin slides in
/// at it or up to half a millimetre over.
fn stock(diameter: f64, sizes: &[f64]) -> Option<f64> {
    sizes
        .iter()
        .copied()
        .filter(|d| diameter > d - NEAR && diameter < d + 0.5)
        .min_by(|a, b| (diameter - a).abs().total_cmp(&(diameter - b).abs()))
}

/// The size `hole` calls for in `part`; empty when it calls for none.
pub fn fit(part: &Hardware, hole: &Hole) -> Fit {
    use ThreadHole::*;
    let thread = |ways: &[ThreadHole]| Fit {
        size: thread_size(hole.diameter, ways).map(String::from),
        ..Fit::default()
    };
    match part {
        Hardware::Insert(_) => thread(&[Insert]),
        Hardware::Screw(_) | Hardware::Nut(_) | Hardware::Washer(_) => {
            thread(&[Clearance, Tap, Insert, Nominal])
        }
        Hardware::Standoff(s) if s.stud > 0.0 => thread(&[Tap, Clearance, Insert, Nominal]),
        Hardware::Standoff(_) => thread(&[Clearance, Tap, Insert, Nominal]),
        Hardware::Rod(r) => {
            let size = match r.kind {
                RodKind::Threaded => {
                    thread_size(hole.diameter, &[Clearance, Tap, Nominal]).map(String::from)
                }
                RodKind::Shaft => stock(hole.diameter, &standards::SHAFT_DIAMETERS)
                    .map(|d| format!("Ø{}", fmt(d))),
                RodKind::Dowel => stock(hole.diameter, &standards::DOWEL_DIAMETERS)
                    .map(|d| format!("Ø{}", fmt(d))),
                RodKind::LeadScrew => None,
            };
            Fit {
                size,
                ..Fit::default()
            }
        }
        Hardware::Bearing(b) => Fit {
            bearing: bearing_for(b.kind, &b.name, hole),
            ..Fit::default()
        },
        Hardware::Magnet(m) if m.shape == MagnetShape::Disc => Fit {
            magnet: magnet_for(hole),
            ..Fit::default()
        },
        _ => Fit::default(),
    }
}

/// The bearing a pocket of the hole's diameter holds: its outer ring at
/// that or a little under, the one already chosen when it fits, else the
/// widest that the pocket is deep enough for.
fn bearing_for(kind: BearingKind, current: &str, hole: &Hole) -> Option<&'static str> {
    let rows = kind.rows();
    let outer = stock(
        hole.diameter,
        &rows.iter().map(|r| r.outer).collect::<Vec<_>>(),
    )?;
    let fits: Vec<_> = rows
        .iter()
        .filter(|r| (r.outer - outer).abs() < 1e-9)
        .collect();
    if let Some(r) = fits.iter().find(|r| r.name == current) {
        return Some(r.name);
    }
    let room = |w: f64| hole.through || w <= hole.depth + NEAR;
    fits.iter()
        .filter(|r| room(r.width))
        .max_by(|a, b| a.width.total_cmp(&b.width))
        .or_else(|| fits.iter().min_by(|a, b| a.width.total_cmp(&b.width)))
        .map(|r| r.name)
}

/// The disc magnet a pocket holds: its diameter, and the tallest of it
/// the pocket is deep enough for.
fn magnet_for(hole: &Hole) -> Option<(f64, f64)> {
    let discs = &standards::DISC_MAGNETS;
    let d = stock(
        hole.diameter,
        &discs.iter().map(|m| m.0).collect::<Vec<_>>(),
    )?;
    let heights = discs.iter().filter(|m| m.0 == d).map(|m| m.1);
    let room = |h: f64| hole.through || h <= hole.depth + NEAR;
    heights
        .clone()
        .filter(|h| room(*h))
        .max_by(f64::total_cmp)
        .or_else(|| heights.min_by(f64::total_cmp))
        .map(|h| (d, h))
}

/// Where along its own Z the part meets the hole's opening, and whether
/// its +Z runs into the hole (else out of it); `None` for a part that
/// does not go in a round hole.
pub fn seat(part: &Hardware, hole: &Hole) -> Option<(f64, bool)> {
    // Pressed in flush with the opening, or to the bottom of a shallower
    // blind hole.
    let flush = |l: f64| if hole.through { l } else { l.min(hole.depth) };
    Some(match part {
        // On the face round the opening, the shank or the stud in it.
        Hardware::Screw(_) | Hardware::Nut(_) | Hardware::Washer(_) | Hardware::Gear(_) => {
            (0.0, false)
        }
        Hardware::Standoff(s) if s.stud > 0.0 => (s.length, true),
        Hardware::Standoff(_) => (0.0, false),
        Hardware::Insert(i) => (flush(i.length), false),
        Hardware::Bearing(b) => (flush(b.width), false),
        Hardware::Magnet(m) => (flush(m.h), false),
        // Down to the bottom, or to the far face of a through hole.
        Hardware::Rod(r) => (r.length.min(hole.depth), false),
        Hardware::Spring(s) => (s.length.min(hole.depth), false),
        Hardware::TNut(_) | Hardware::Extrusion(_) => return None,
    })
}

/// The placement (row-major 4x4) of the part's body that sits it in
/// `hole`.
pub fn placement(part: &Hardware, hole: &Hole) -> Option<[f64; 16]> {
    let (s, inward) = seat(part, hole)?;
    let z = if inward {
        hole.direction
    } else {
        scale(hole.direction, -1.0)
    };
    // Any X square to it, kept near the world's X so a row of parts in
    // like holes turn alike.
    let helper = if z[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let x = unit(sub(helper, scale(z, dot(helper, z))))?;
    let y = cross(z, x);
    let t = sub(hole.entry, scale(z, s));
    Some([
        x[0], y[0], z[0], t[0], //
        x[1], y[1], z[1], t[1], //
        x[2], y[2], z[2], t[2], //
        0.0, 0.0, 0.0, 1.0,
    ])
}

fn f64s(v: [f32; 3]) -> [f64; 3] {
    v.map(f64::from)
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: [f64; 3], k: f64) -> [f64; 3] {
    a.map(|v| v * k)
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn len(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let l = len(a);
    (l > 1e-12).then(|| scale(a, 1.0 / l))
}

/// A part's own point taken where `m` (row-major) puts it.
pub fn transform(m: &[f64; 16], p: [f64; 3]) -> [f64; 3] {
    [
        m[0] * p[0] + m[1] * p[1] + m[2] * p[2] + m[3],
        m[4] * p[0] + m[5] * p[1] + m[6] * p[2] + m[7],
        m[8] * p[0] + m[9] * p[1] + m[10] * p[2] + m[11],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parts::bearing::BearingKind;
    use crate::parts::screw::Head;
    use crate::parts::{Bearing, Insert, Magnet, Rod, Screw, Standoff};
    use printcad_bench_sdk::api::EdgeCircle;

    /// A blind hole down into the top face (z = 10) of a plate at
    /// (x, 0).
    fn hole(x: f64, diameter: f64, depth: f64) -> Hole {
        Hole {
            entry: [x, 0.0, 10.0],
            direction: [0.0, 0.0, -1.0],
            diameter,
            depth,
            through: false,
        }
    }

    fn bore(x: f32, radius: f32, z: f32) -> FaceRef {
        FaceRef {
            point: [x + radius, 0.0, z],
            normal: [-1.0, 0.0, 0.0],
            surface: Some(FaceSurface::Cylinder {
                origin: [x, 0.0, 0.0],
                axis: [0.0, 0.0, 1.0],
                radius,
            }),
            name: None,
        }
    }

    fn near(a: [f64; 3], b: [f64; 3]) -> bool {
        len(sub(a, b)) < 1e-9
    }

    #[test]
    fn a_hole_reads_from_the_hosts_answer() {
        let v = json!({"entry": [1.0, 2.0, 3.0], "direction": [0.0, 0.0, -2.0], "diameter": 3.4, "depth": 6.0, "through": false, "drill_point_deg": null});
        let h = Hole::read(&v).unwrap();
        assert_eq!(h.direction, [0.0, 0.0, -1.0], "made a unit vector");
        assert_eq!(Hole::read(&h.to_value()), Some(h));
    }

    #[test]
    fn a_picked_bore_names_its_hole() {
        let holes = [
            hole(0.0, 3.4, 5.0),
            hole(20.0, 3.4, 5.0),
            hole(20.0, 6.0, 2.0),
        ];
        let pointer = |faces: Vec<FaceRef>| Pointer {
            faces,
            ..Pointer::default()
        };
        let picked = |holes: &[Hole], p: &Pointer| -> Vec<usize> {
            picked(holes, p, None).into_iter().map(|(i, _)| i).collect()
        };
        assert_eq!(picked(&holes, &pointer(vec![bore(20.0, 1.7, 8.0)])), [1]);
        assert_eq!(
            picked(&holes, &pointer(vec![bore(20.0, 3.0, 9.0)])),
            [2],
            "the wider bore on the same axis"
        );
        assert_eq!(
            picked(
                &holes,
                &pointer(vec![
                    bore(20.0, 1.7, 8.0),
                    bore(0.0, 1.7, 8.0),
                    bore(20.0, 1.7, 7.0)
                ])
            ),
            [1, 0],
            "in the order picked, each once"
        );
        assert!(picked(&holes, &pointer(vec![bore(40.0, 1.7, 8.0)])).is_empty());
    }

    /// The host names a through hole from either opening; the part goes
    /// in from the one picked, or in view.
    #[test]
    fn a_through_hole_runs_from_the_opening_picked() {
        // Through a plate from z = 0 to 8, named from below.
        let holes = [Hole {
            entry: [0.0, 0.0, 0.0],
            direction: [0.0, 0.0, 1.0],
            diameter: 3.4,
            depth: 8.0,
            through: true,
        }];
        let rim = |z: f32| EdgeRef {
            body: "b".into(),
            point: [1.7, 0.0, z],
            direction: [0.0, 1.0, 0.0],
            length: 10.7,
            circle: Some(EdgeCircle {
                center: [0.0, 0.0, z],
                normal: [0.0, 0.0, 1.0],
                radius: 1.7,
            }),
            faces: None,
        };
        let from = |p: Pointer, toward| {
            let [(0, h)] = &picked(&holes, &p, toward)[..] else {
                panic!("one hole")
            };
            (h.entry, h.direction)
        };
        let top = from(
            Pointer {
                edges: vec![rim(8.0)],
                ..Pointer::default()
            },
            None,
        );
        assert_eq!(top, ([0.0, 0.0, 8.0], [0.0, 0.0, -1.0]), "its top rim");
        let bottom = from(
            Pointer {
                edges: vec![rim(0.0)],
                ..Pointer::default()
            },
            None,
        );
        assert_eq!(bottom, ([0.0, 0.0, 0.0], [0.0, 0.0, 1.0]), "its bottom rim");
        let face = Pointer {
            faces: vec![bore(0.0, 1.7, 4.0)],
            ..Pointer::default()
        };
        assert_eq!(
            from(face.clone(), None).0,
            [0.0, 0.0, 8.0],
            "seen from above"
        );
        assert_eq!(
            from(face, Some([0.2, 0.0, 1.0])).0,
            [0.0, 0.0, 0.0],
            "seen from below"
        );
    }

    #[test]
    fn a_bore_the_host_does_not_count_runs_from_the_end_in_view() {
        // A Ø3.4 bore from z = 2 to z = 8 under a countersink.
        let face = bore(0.0, 1.7, 5.0);
        let ends = [[0.0, 0.0, 8.0], [0.0, 0.0, 2.0], [9.0, 0.0, 8.0]];
        let h = super::bore(&face, &ends, Some([0.0, 0.3, -1.0])).unwrap();
        assert!(near(h.entry, [0.0, 0.0, 8.0]) && near(h.direction, [0.0, 0.0, -1.0]));
        assert!((h.depth - 6.0).abs() < 1e-9 && (h.diameter - 3.4).abs() < 1e-6);
        let below = super::bore(&face, &ends, Some([0.0, 0.0, 1.0])).unwrap();
        assert!(near(below.entry, [0.0, 0.0, 2.0]), "seen from below");
        let shaft = FaceRef {
            normal: [1.0, 0.0, 0.0],
            ..face
        };
        assert_eq!(super::bore(&shaft, &ends, None), None, "a shaft is no bore");
    }

    #[test]
    fn a_counterbores_seat_or_a_rim_names_the_hole_under_it() {
        // A Ø3.4 bore down from a seat at z = 7, under a Ø6.5 counterbore.
        let holes = [
            hole(0.0, 6.5, 3.0),
            Hole {
                entry: [0.0, 0.0, 7.0],
                ..hole(0.0, 3.4, 4.0)
            },
        ];
        let seat = FaceRef {
            point: [2.5, 0.0, 7.0],
            normal: [0.0, 0.0, 1.0],
            surface: Some(FaceSurface::Plane {
                origin: [0.0, 0.0, 7.0],
                normal: [0.0, 0.0, 1.0],
            }),
            name: None,
        };
        assert_eq!(at_face(&holes, &seat), Some(1));
        let rim = EdgeRef {
            body: "b".into(),
            point: [1.7, 0.0, 7.0],
            direction: [0.0, 1.0, 0.0],
            length: 10.7,
            circle: Some(EdgeCircle {
                center: [0.0, 0.0, 7.0],
                normal: [0.0, 0.0, 1.0],
                radius: 1.7,
            }),
            faces: None,
        };
        assert_eq!(at_edge(&holes, &rim), Some(1));
        let top = EdgeRef {
            circle: Some(EdgeCircle {
                center: [0.0, 0.0, 10.0],
                normal: [0.0, 0.0, -1.0],
                radius: 3.25,
            }),
            ..rim
        };
        assert_eq!(at_edge(&holes, &top), Some(0));
    }

    #[test]
    fn a_hole_sizes_the_thread_it_was_made_for() {
        let d = Defaults::default();
        let screw = Hardware::Screw(Screw::new(Head::SocketCap, "M3", &d));
        let size = |part: &Hardware, dia: f64| fit(part, &hole(0.0, dia, 10.0)).size;
        assert_eq!(size(&screw, 4.5).as_deref(), Some("M4"), "clearance");
        assert_eq!(size(&screw, 5.4).as_deref(), Some("M5"));
        assert_eq!(size(&screw, 5.0).as_deref(), Some("M6"), "tap drill");
        assert_eq!(size(&screw, 3.4).as_deref(), Some("M3"));
        assert_eq!(size(&screw, 30.0), None);
        let insert = Hardware::Insert(Insert::new("M2", &d));
        assert_eq!(
            size(&insert, 4.0).as_deref(),
            Some("M3"),
            "an insert's hole"
        );
        assert_eq!(size(&insert, 5.6).as_deref(), Some("M4"));
        let resized = fit(&screw, &hole(0.0, 6.6, 10.0)).apply(screw.clone(), &d);
        let Hardware::Screw(s) = resized else {
            unreachable!()
        };
        assert_eq!((s.size.as_str(), s.head), ("M6", Head::SocketCap));
    }

    #[test]
    fn a_pocket_sizes_its_bearing_magnet_or_shaft() {
        let d = Defaults::default();
        let bearing = Hardware::Bearing(Bearing::new(BearingKind::Ball, "608"));
        assert_eq!(fit(&bearing, &hole(0.0, 22.1, 7.0)).bearing, Some("608"));
        // 19 mm: 626, 698, 6800 and 607 share it; a 5 mm pocket takes the
        // 6800.
        assert_eq!(fit(&bearing, &hole(0.0, 19.1, 5.0)).bearing, Some("6800"));
        let magnet = Hardware::Magnet(Magnet::new(MagnetShape::Disc));
        assert_eq!(fit(&magnet, &hole(0.0, 6.2, 2.5)).magnet, Some((6.0, 2.0)));
        assert_eq!(fit(&magnet, &hole(0.0, 6.2, 4.0)).magnet, Some((6.0, 3.0)));
        let shaft = Hardware::Rod(Rod::new(RodKind::Shaft, "Ø8", &d));
        assert_eq!(
            fit(&shaft, &hole(0.0, 10.2, 20.0)).size.as_deref(),
            Some("Ø10")
        );
    }

    #[test]
    fn a_commands_own_size_is_kept() {
        let f = Fit {
            size: Some("M4".into()),
            ..Fit::default()
        };
        assert_eq!(f.into_args(&json!({"part": "screw"}))["size"], "M4");
        assert_eq!(f.into_args(&json!({"size": "M3"}))["size"], "M3");
        assert_eq!(f.into_args(&Value::Null)["size"], "M4");
    }

    #[test]
    fn a_screw_stands_on_the_opening_its_shank_down_the_hole() {
        let d = Defaults::default();
        let screw = Hardware::Screw(Screw::new(Head::SocketCap, "M3", &d));
        let h = hole(5.0, 3.4, 8.0);
        let m = placement(&screw, &h).unwrap();
        assert!(near(transform(&m, [0.0; 3]), h.entry), "under the head");
        assert!(
            near(transform(&m, [0.0, 0.0, -4.0]), [5.0, 0.0, 6.0]),
            "the shank runs into the hole"
        );
    }

    #[test]
    fn an_insert_sits_flush_and_a_stud_goes_in() {
        let d = Defaults::default();
        let insert = Hardware::Insert(Insert::new("M3", &d));
        let Hardware::Insert(i) = &insert else {
            unreachable!()
        };
        // Any way the hole runs: along X, out of a side face at x = 0.
        let h = Hole {
            entry: [0.0, 3.0, 4.0],
            direction: [1.0, 0.0, 0.0],
            diameter: 4.0,
            depth: 8.0,
            through: false,
        };
        let m = placement(&insert, &h).unwrap();
        assert!(near(transform(&m, [0.0, 0.0, i.length]), h.entry), "flush");
        assert!(near(transform(&m, [0.0; 3]), [i.length, 3.0, 4.0]));
        let shallow = Hole {
            depth: 3.0,
            ..h.clone()
        };
        let m = placement(&insert, &shallow).unwrap();
        assert!(
            near(transform(&m, [0.0; 3]), [3.0, 3.0, 4.0]),
            "on a shallow hole's floor"
        );

        let mut standoff = Standoff::new(crate::parts::standoff::StandoffShape::Hex, "M3", &d);
        standoff.stud = 6.0;
        let standoff = Hardware::Standoff(standoff);
        let m = placement(&standoff, &hole(0.0, 2.5, 8.0)).unwrap();
        assert!(
            near(transform(&m, [0.0, 0.0, 16.0]), [0.0, 0.0, 4.0]),
            "the stud's tip in the hole"
        );
        assert!(
            near(transform(&m, [0.0; 3]), [0.0, 0.0, 20.0]),
            "the body above"
        );
    }

    #[test]
    fn every_placement_is_a_rigid_motion() {
        let d = Defaults::default();
        let screw = Hardware::Screw(Screw::new(Head::SocketCap, "M3", &d));
        for dir in [
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.6, 0.0, 0.8],
        ] {
            let h = Hole {
                direction: dir,
                ..hole(0.0, 3.4, 5.0)
            };
            let m = placement(&screw, &h).unwrap();
            let col = |j: usize| [m[j], m[4 + j], m[8 + j]];
            for (a, b) in [(0, 1), (1, 2), (0, 2)] {
                assert!(dot(col(a), col(b)).abs() < 1e-9);
            }
            assert!(near(cross(col(0), col(1)), col(2)), "right-handed");
            assert!(near(col(2), scale(dir, -1.0)));
        }
    }
}
