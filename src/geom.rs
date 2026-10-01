//! The kernel operations the parts are built from: bodies of revolution
//! from a half-section, prisms from an outline, threads cut along a
//! helix, and the planes they stand on.
//!
//! Every part stands on its axis along Z. A half-section is a closed
//! polygon in `(r, z)`, `r` out from the axis; a prism's outline is a
//! closed polygon in the XY plane, extruded up Z.

use std::f64::consts::{PI, TAU};

use printcad_bench_sdk::api::kernel_api::{
    BooleanOp, ExtrudeTermination, Profile, ProfilePlane, ProfileSegment, ProfileWire, SolidOp,
    SweepKind,
};

/// The XY plane, extruding up Z.
pub fn xy(z: f64) -> ProfilePlane {
    ProfilePlane {
        origin: [0.0, 0.0, z],
        x_axis: [1.0, 0.0, 0.0],
        y_axis: [0.0, 1.0, 0.0],
        normal: [0.0, 0.0, 1.0],
    }
}

/// The XY plane at `z`, extruding down.
pub fn xy_down(z: f64) -> ProfilePlane {
    ProfilePlane {
        origin: [0.0, 0.0, z],
        x_axis: [1.0, 0.0, 0.0],
        y_axis: [0.0, -1.0, 0.0],
        normal: [0.0, 0.0, -1.0],
    }
}

/// The half-plane a body of revolution is drawn in: x out from the axis
/// (world X), y up it (world Z).
pub fn section() -> ProfilePlane {
    ProfilePlane {
        origin: [0.0; 3],
        x_axis: [1.0, 0.0, 0.0],
        y_axis: [0.0, 0.0, 1.0],
        normal: [0.0, -1.0, 0.0],
    }
}

/// A closed polygon's segments, coincident corners dropped.
pub fn polygon(points: &[[f64; 2]]) -> Vec<ProfileSegment> {
    let mut out = Vec::with_capacity(points.len());
    for (i, start) in points.iter().enumerate() {
        let end = points[(i + 1) % points.len()];
        if (start[0] - end[0]).abs() > 1e-9 || (start[1] - end[1]).abs() > 1e-9 {
            out.push(ProfileSegment::Line { start: *start, end });
        }
    }
    out
}

/// A circle about `center`, of diameter `d`.
pub fn circle_at(center: [f64; 2], d: f64) -> ProfileSegment {
    ProfileSegment::Circle {
        center,
        radius: d / 2.0,
    }
}

pub fn circle(d: f64) -> ProfileSegment {
    circle_at([0.0, 0.0], d)
}

/// A regular polygon of `sides` about the origin, a flat square to +X,
/// `across` wide between opposite flats.
pub fn regular(sides: u32, across: f64) -> Vec<[f64; 2]> {
    let n = sides.max(3);
    let r = across / 2.0 / (PI / n as f64).cos();
    let half = PI / n as f64;
    (0..n)
        .map(|i| {
            let a = half + TAU * i as f64 / n as f64;
            [r * a.cos(), r * a.sin()]
        })
        .collect()
}

/// A hexagon `across` flats, as a nut or a head is.
pub fn hexagon(across: f64) -> Vec<[f64; 2]> {
    regular(6, across)
}

/// A hexagon's width across corners.
pub fn across_corners(across_flats: f64) -> f64 {
    across_flats / (PI / 6.0).cos()
}

/// A square of side `s` about the origin.
pub fn square(s: f64) -> Vec<[f64; 2]> {
    let h = s / 2.0;
    vec![[-h, -h], [h, -h], [h, h], [-h, h]]
}

/// A rectangle about the origin.
pub fn rectangle(w: f64, h: f64) -> Vec<[f64; 2]> {
    let (x, y) = (w / 2.0, h / 2.0);
    vec![[-x, -y], [x, -y], [x, y], [-x, y]]
}

/// Extrude `wires` on `plane` by `height`; the first wire is the outside.
pub fn extrude(
    plane: ProfilePlane,
    wires: Vec<Vec<ProfileSegment>>,
    height: f64,
    op: BooleanOp,
) -> SolidOp {
    SolidOp::Sweep {
        profile: Profile {
            plane,
            wires: wires.into_iter().map(ProfileWire::new).collect(),
        },
        kind: SweepKind::Extrude {
            termination: ExtrudeTermination::Blind { distance: height },
            second_side: None,
            symmetric: false,
            reversed: false,
            taper_deg: 0.0,
            direction: None,
        },
        op,
    }
}

/// A prism of `outline` from `z` up `height`, with `holes` through it.
pub fn prism(
    outline: &[[f64; 2]],
    holes: Vec<ProfileSegment>,
    z: f64,
    height: f64,
    op: BooleanOp,
) -> SolidOp {
    let mut wires = vec![polygon(outline)];
    wires.extend(holes.into_iter().map(|h| vec![h]));
    extrude(xy(z), wires, height, op)
}

/// Turn a half-section in `(r, z)` about the Z axis. The polygon may run
/// along the axis (`r = 0`); it must not cross it.
pub fn revolve(section_points: &[[f64; 2]], op: BooleanOp) -> SolidOp {
    revolve_segments(polygon(section_points), op)
}

/// Turn a half-section drawn as segments (lines and arcs) about Z.
pub fn revolve_segments(segments: Vec<ProfileSegment>, op: BooleanOp) -> SolidOp {
    SolidOp::Sweep {
        profile: Profile {
            plane: section(),
            wires: vec![ProfileWire::new(segments)],
        },
        kind: SweepKind::Revolve {
            axis_origin: [0.0, 0.0],
            axis_dir: [0.0, 1.0],
            angle_deg: 360.0,
            second_angle_deg: None,
            midplane: false,
            reversed: false,
            termination: Default::default(),
        },
        op,
    }
}

/// The envelope of a chamfered cylinder: diameter `d` between `z0` and
/// `z1`, its ends cut at `angle_deg` to the face, `c` deep along the
/// axis, at the bottom when `bottom` and the top when `top`. Taken
/// `Common` with a prism, it crowns a nut or a hex head.
pub fn crown(d: f64, z0: f64, z1: f64, c: f64, angle_deg: f64, bottom: bool, top: bool) -> SolidOp {
    let r = d / 2.0 + 0.01;
    let inset = c / angle_deg.to_radians().tan();
    let mut pts = vec![[0.0, z0]];
    if bottom {
        pts.push([r - inset, z0]);
        pts.push([r, z0 + c]);
    } else {
        pts.push([r, z0]);
    }
    if top {
        pts.push([r, z1 - c]);
        pts.push([r - inset, z1]);
    } else {
        pts.push([r, z1]);
    }
    pts.push([0.0, z1]);
    revolve(&pts, BooleanOp::Common)
}

/// A hex socket `s` across flats cut `t` deep down from `z`. The cut
/// starts a millimetre above the face, so a domed head is pierced rather
/// than met at a point.
pub fn hex_socket(s: f64, t: f64, z: f64) -> SolidOp {
    extrude(
        xy_down(z + 1.0),
        vec![polygon(&hexagon(s))],
        t + 1.0,
        BooleanOp::Cut,
    )
}

/// A thread's groove, cut along a helix. `open` is the radius the groove
/// opens at (a screw's outside, a nut's bore) and `bottom` the radius it
/// reaches; it is wide at the opening and nearly sharp at the bottom, at
/// 60° flanks. It runs from `z_top` down `length` and a pitch past, out
/// through the end. With `lead_in` it starts a pitch above, so the groove
/// enters a plain shank cleanly; without, half a pitch down, so its first
/// turn stays within the face it starts under.
pub fn thread_groove(
    open: f64,
    bottom: f64,
    pitch: f64,
    z_top: f64,
    length: f64,
    lead_in: bool,
    left_handed: bool,
) -> SolidOp {
    // The section, in a plane through the axis: x out from it, y down it.
    let plane = ProfilePlane {
        origin: [0.0, 0.0, z_top],
        x_axis: [1.0, 0.0, 0.0],
        y_axis: [0.0, 0.0, -1.0],
        normal: [0.0, 1.0, 0.0],
    };
    let outward = open > bottom;
    // Well past the opening, so the groove leaves no skin.
    let over = if outward {
        open + 0.5 * pitch
    } else {
        (open - 0.5 * pitch).max(0.1 * open)
    };
    let crest = pitch / 16.0;
    let root = (crest + (open - bottom).abs() * (30f64).to_radians().tan()).min(0.45 * pitch);
    let start = if lead_in { -pitch } else { 0.5 * pitch };
    let corners = [
        [over, start - root],
        [bottom, start - crest],
        [bottom, start + crest],
        [over, start + root],
    ];
    SolidOp::Sweep {
        profile: Profile {
            plane,
            wires: vec![ProfileWire::new(polygon(&corners))],
        },
        kind: SweepKind::Helix {
            axis_origin: [0.0, 0.0],
            axis_dir: [0.0, 1.0],
            pitch,
            height: length + pitch - start,
            left_handed,
            cone_angle_deg: 0.0,
            reversed: false,
            turns: None,
            growth: None,
        },
        op: BooleanOp::Cut,
    }
}

/// An external thread of major diameter `d` on a shank, from `z_top`
/// down `length`.
pub fn external_thread(d: f64, pitch: f64, z_top: f64, length: f64, lead_in: bool) -> SolidOp {
    thread_groove(
        d / 2.0,
        d / 2.0 - 0.6134 * pitch,
        pitch,
        z_top,
        length,
        lead_in,
        false,
    )
}

/// An internal thread in a bore of minor diameter `minor`, out to major
/// `d`, through a nut from `z_top` down `length`.
pub fn internal_thread(d: f64, minor: f64, pitch: f64, z_top: f64, length: f64) -> SolidOp {
    thread_groove(minor / 2.0, d / 2.0, pitch, z_top, length, true, false)
}

/// Points along an arc from angle `a0` to `a1` about `center`, for
/// drawing (not for the kernel, which takes an arc as three points).
pub fn arc_points(center: [f64; 2], r: f64, a0: f64, a1: f64, steps: usize) -> Vec<[f64; 2]> {
    (0..=steps)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f64 / steps as f64;
            [center[0] + r * a.cos(), center[1] + r * a.sin()]
        })
        .collect()
}

/// The kernel's arc through `start` and `end` on the circle about
/// `center` of radius `r`, taking the shorter way round.
pub fn arc(center: [f64; 2], r: f64, start: [f64; 2], end: [f64; 2]) -> ProfileSegment {
    let a0 = (start[1] - center[1]).atan2(start[0] - center[0]);
    let mut a1 = (end[1] - center[1]).atan2(end[0] - center[0]);
    while a1 - a0 > PI {
        a1 -= TAU;
    }
    while a0 - a1 > PI {
        a1 += TAU;
    }
    let am = (a0 + a1) / 2.0;
    ProfileSegment::Arc {
        start,
        mid: [center[0] + r * am.cos(), center[1] + r * am.sin()],
        end,
    }
}

/// A number for a label, without trailing zeros.
pub fn trim(v: f64) -> String {
    let text = format!("{v:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hexagon_is_as_wide_across_its_flats_as_asked() {
        let hex = hexagon(10.0);
        assert_eq!(hex.len(), 6);
        let max_x = hex.iter().map(|p| p[0].abs()).fold(0.0, f64::max);
        assert!((2.0 * max_x - 10.0).abs() < 1e-9, "{}", 2.0 * max_x);
        // Flats square to X: two corners lie on the right flat.
        let on_right = hex.iter().filter(|p| (p[0] - 5.0).abs() < 1e-9).count();
        assert_eq!(on_right, 2);
        assert!((across_corners(10.0) - 11.547).abs() < 1e-3);
    }

    #[test]
    fn a_square_is_square() {
        let sq = regular(4, 8.0);
        for p in &sq {
            assert!((p[0].abs() - 4.0).abs() < 1e-9 && (p[1].abs() - 4.0).abs() < 1e-9);
        }
    }

    #[test]
    fn a_polygon_drops_a_repeated_corner() {
        let segs = polygon(&[[0.0, 0.0], [1.0, 0.0], [1.0, 0.0], [1.0, 1.0]]);
        assert_eq!(segs.len(), 3);
    }

    #[test]
    fn a_thread_groove_opens_where_it_should() {
        let SolidOp::Sweep { profile, kind, op } = external_thread(3.0, 0.5, 0.0, 10.0, true)
        else {
            panic!("a sweep");
        };
        assert_eq!(op, BooleanOp::Cut);
        let SweepKind::Helix { pitch, height, .. } = kind else {
            panic!("a helix");
        };
        assert_eq!(pitch, 0.5);
        assert!(
            (height - 11.0).abs() < 1e-9,
            "a pitch above and a pitch past the tip: {height}"
        );
        let xs: Vec<f64> = profile.wires[0]
            .segments
            .iter()
            .filter_map(|s| match s {
                ProfileSegment::Line { start, .. } => Some(start[0]),
                _ => None,
            })
            .collect();
        assert!(
            xs.iter().cloned().fold(f64::MIN, f64::max) > 1.5,
            "past the outside"
        );
        assert!(
            xs.iter().cloned().fold(f64::MAX, f64::min) < 1.5,
            "into the shank"
        );
    }

    #[test]
    fn an_arc_takes_the_short_way() {
        let ProfileSegment::Arc { mid, .. } = arc([0.0, 0.0], 1.0, [1.0, 0.0], [0.0, 1.0]) else {
            panic!("an arc");
        };
        let expected = std::f64::consts::FRAC_1_SQRT_2;
        assert!((mid[0] - expected).abs() < 1e-9 && (mid[1] - expected).abs() < 1e-9);
    }
}
