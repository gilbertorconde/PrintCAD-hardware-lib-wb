//! Gears: involute spur, helical and herringbone gears, internal ring
//! gears, racks, worms, straight bevel gears and GT2 timing pulleys, on
//! `z = 0` with their axis up Z (a rack runs along X). Any gear takes a
//! shaft hole of the common forms, a hub, and a set screw in the hub.

use std::f64::consts::{PI, TAU};

use printcad_bench_sdk::Value;
use printcad_bench_sdk::api::kernel_api::{
    BooleanOp, Profile, ProfilePlane, ProfileSegment, ProfileWire, SolidOp,
};
use printcad_bench_sdk::api::{DiagramStroke, PanelEvent, Parameter, Widget};
use serde::{Deserialize, Serialize};

use super::{
    Ctx, Defaults, Family, Part, arg_bool, arg_f64, arg_str, choice, count, fmt, group, integer,
    length, note, number, toggle,
};
use crate::diagram::Sketch;
use crate::geom;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GearKind {
    Spur,
    Helical,
    Herringbone,
    Internal,
    Rack,
    Worm,
    Bevel,
    Pulley,
}

impl GearKind {
    pub const ALL: [GearKind; 8] = [
        GearKind::Spur,
        GearKind::Helical,
        GearKind::Herringbone,
        GearKind::Internal,
        GearKind::Rack,
        GearKind::Worm,
        GearKind::Bevel,
        GearKind::Pulley,
    ];

    pub fn name(self) -> &'static str {
        match self {
            GearKind::Spur => "Spur gear",
            GearKind::Helical => "Helical gear",
            GearKind::Herringbone => "Herringbone gear",
            GearKind::Internal => "Internal ring gear",
            GearKind::Rack => "Rack",
            GearKind::Worm => "Worm",
            GearKind::Bevel => "Bevel gear",
            GearKind::Pulley => "GT2 timing pulley",
        }
    }

    fn short(self) -> &'static str {
        match self {
            GearKind::Spur => "spur gear",
            GearKind::Helical => "helical gear",
            GearKind::Herringbone => "herringbone gear",
            GearKind::Internal => "ring gear",
            GearKind::Rack => "rack",
            GearKind::Worm => "worm",
            GearKind::Bevel => "bevel gear",
            GearKind::Pulley => "GT2 pulley",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            GearKind::Spur => "gear-spur",
            GearKind::Helical => "gear-helical",
            GearKind::Herringbone => "gear-herringbone",
            GearKind::Internal => "gear-internal",
            GearKind::Rack => "gear-rack",
            GearKind::Worm => "gear-worm",
            GearKind::Bevel => "gear-bevel",
            GearKind::Pulley => "gear-pulley",
        }
    }

    pub fn tool(self) -> &'static str {
        match self {
            GearKind::Spur => "spur_gear",
            GearKind::Helical => "helical_gear",
            GearKind::Herringbone => "herringbone_gear",
            GearKind::Internal => "internal_gear",
            GearKind::Rack => "rack",
            GearKind::Worm => "worm",
            GearKind::Bevel => "bevel_gear",
            GearKind::Pulley => "gt2_pulley",
        }
    }

    pub fn named(name: &str) -> Option<GearKind> {
        let key = name.to_lowercase().replace(['-', ' '], "_");
        GearKind::ALL.into_iter().find(|k| {
            k.tool() == key
                || k.tool().trim_end_matches("_gear") == key
                || k.name().eq_ignore_ascii_case(name)
                || (key == "ring" && *k == GearKind::Internal)
                || (key == "pulley" && *k == GearKind::Pulley)
                || (key == "timing_pulley" && *k == GearKind::Pulley)
        })
    }

    /// Whether the gear is round with teeth on its outside: what takes
    /// a hub and whose teeth can be helical.
    fn is_wheel(self) -> bool {
        matches!(
            self,
            GearKind::Spur | GearKind::Helical | GearKind::Herringbone | GearKind::Bevel
        )
    }

    /// Whether a shaft hole is cut.
    fn has_bore(self) -> bool {
        !matches!(self, GearKind::Rack | GearKind::Internal)
    }
}

/// The form of the shaft hole.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shaft {
    Round,
    /// One flat, `flat` across from the opposite side.
    D,
    /// Two flats, `flat` apart.
    DoubleD,
    /// A hexagon, the bore across flats.
    Hex,
    /// A keyway `key_w` wide and `key_d` deep.
    Keyed,
}

impl Shaft {
    pub const ALL: [Shaft; 5] = [
        Shaft::Round,
        Shaft::D,
        Shaft::DoubleD,
        Shaft::Hex,
        Shaft::Keyed,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Shaft::Round => "Round",
            Shaft::D => "D (one flat)",
            Shaft::DoubleD => "Double D",
            Shaft::Hex => "Hex",
            Shaft::Keyed => "Keyed",
        }
    }

    pub fn tool(self) -> &'static str {
        match self {
            Shaft::Round => "round",
            Shaft::D => "d",
            Shaft::DoubleD => "dd",
            Shaft::Hex => "hex",
            Shaft::Keyed => "keyed",
        }
    }
}

/// A shaft the makers fit motors and bearings with: name, bore, form,
/// across the flat(s).
pub const SHAFTS: [(&str, f64, Shaft, f64); 8] = [
    ("Custom", 0.0, Shaft::Round, 0.0),
    ("NEMA 17 (Ø5 D)", 5.0, Shaft::D, 4.5),
    ("NEMA 23 (Ø6.35 D)", 6.35, Shaft::D, 5.85),
    ("28BYJ-48 (Ø5 double D)", 5.0, Shaft::DoubleD, 3.0),
    ("N20 (Ø3 D)", 3.0, Shaft::D, 2.5),
    ("TT motor (Ø5.4 double D)", 5.4, Shaft::DoubleD, 3.7),
    ("608 bearing (Ø8)", 8.0, Shaft::Round, 0.0),
    ("T8 lead screw (Ø8)", 8.0, Shaft::Round, 0.0),
];

/// The GT2 belt: 2 mm pitch, the pitch line 0.254 out from the pulley's
/// tips, teeth 0.75 deep.
const GT2_PITCH: f64 = 2.0;
const GT2_PLD: f64 = 0.254;
const GT2_DEPTH: f64 = 0.75;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gear {
    pub kind: GearKind,
    pub teeth: u32,
    /// Millimetres of pitch diameter per tooth.
    pub module: f64,
    /// The pressure angle, degrees.
    pub pressure: f64,
    /// Face width up the axis; a rack's width across; a pulley's toothed
    /// width.
    pub thickness: f64,
    /// Thinning of each tooth at the pitch line, shared by the pair.
    #[serde(default)]
    pub backlash: f64,
    /// A helical or herringbone gear's helix angle, degrees, and hand;
    /// a worm's hand.
    #[serde(default)]
    pub helix: f64,
    #[serde(default)]
    pub left: bool,
    /// The teeth of the gear it runs with, for its centre distance; a
    /// worm's wheel; a bevel gear's mate at 90°.
    #[serde(default)]
    pub mate: u32,
    /// The shaft hole: its diameter (a hex's across flats), form, the
    /// flat(s) across, a keyway's width and depth.
    pub bore: f64,
    #[serde(default = "round")]
    pub shaft: Shaft,
    #[serde(default)]
    pub flat: f64,
    #[serde(default)]
    pub key_w: f64,
    #[serde(default)]
    pub key_d: f64,
    /// A hub on top: its diameter (0 for none) and height.
    #[serde(default)]
    pub hub: f64,
    #[serde(default)]
    pub hub_h: f64,
    /// A set screw's thread diameter in the hub: 0 for none.
    #[serde(default)]
    pub set_screw: f64,
    /// A rack's or a worm's length along its axis.
    #[serde(default)]
    pub length: f64,
    /// A ring gear's rim outside its teeth's roots.
    #[serde(default)]
    pub rim: f64,
    /// A worm's pitch diameter and starts.
    #[serde(default)]
    pub worm_d: f64,
    #[serde(default = "one")]
    pub starts: u32,
    /// A pulley's flanges, a millimetre each.
    #[serde(default)]
    pub flanges: bool,
}

fn round() -> Shaft {
    Shaft::Round
}

fn one() -> u32 {
    1
}

impl Gear {
    pub fn new(kind: GearKind) -> Gear {
        let mut gear = Gear {
            kind,
            teeth: 20,
            module: 2.0,
            pressure: 20.0,
            thickness: 8.0,
            backlash: 0.0,
            helix: 0.0,
            left: false,
            mate: 20,
            bore: 5.0,
            shaft: Shaft::D,
            flat: 4.5,
            key_w: 2.0,
            key_d: 1.0,
            hub: 0.0,
            hub_h: 6.0,
            set_screw: 0.0,
            length: 60.0,
            rim: 4.0,
            worm_d: 12.0,
            starts: 1,
            flanges: true,
        };
        if kind == GearKind::Pulley {
            // A 6 mm belt's pulley, with the hub its set screw needs.
            gear.thickness = 7.0;
            gear.hub = 12.0;
            gear.hub_h = 6.0;
            gear.set_screw = 3.0;
        }
        gear.suit();
        gear
    }

    /// Set what a kind needs that the last kind did not.
    fn suit(&mut self) {
        match self.kind {
            GearKind::Helical | GearKind::Herringbone => {
                if self.helix <= 0.0 {
                    self.helix = 20.0;
                }
            }
            GearKind::Worm => {
                if self.length < 10.0 {
                    self.length = 30.0;
                }
            }
            GearKind::Rack => {
                if self.length < 10.0 {
                    self.length = 60.0;
                }
            }
            GearKind::Pulley => {
                if !(3.0..=12.0).contains(&self.thickness) {
                    self.thickness = 7.0;
                }
                if self.hub <= 0.0 {
                    self.hub = 12.0;
                    self.hub_h = 6.0;
                    self.set_screw = 3.0;
                }
            }
            _ => {}
        }
    }

    pub fn pitch_radius(&self) -> f64 {
        match self.kind {
            GearKind::Pulley => GT2_PITCH * f64::from(self.teeth) / TAU,
            GearKind::Worm => self.worm_d / 2.0,
            _ => self.module * f64::from(self.teeth) / 2.0,
        }
    }

    /// The radius over the teeth (a ring gear's over its rim).
    pub fn outer_radius(&self) -> f64 {
        match self.kind {
            GearKind::Pulley => self.pitch_radius() - GT2_PLD,
            GearKind::Internal => self.pitch_radius() + 1.25 * self.module + self.rim,
            GearKind::Worm => self.worm_d / 2.0 + self.module,
            _ => self.pitch_radius() + self.module,
        }
    }

    /// The radius at the teeth's roots.
    fn root_radius(&self) -> f64 {
        match self.kind {
            GearKind::Pulley => self.outer_radius() - GT2_DEPTH,
            GearKind::Internal => self.pitch_radius() - self.module,
            GearKind::Worm => self.worm_d / 2.0 - 1.25 * self.module,
            _ => self.pitch_radius() - 1.25 * self.module,
        }
    }

    /// The centre distance to the mate, when the kind runs on one.
    pub fn centre_distance(&self) -> Option<f64> {
        match self.kind {
            GearKind::Spur | GearKind::Helical | GearKind::Herringbone => {
                Some(self.module * f64::from(self.teeth + self.mate) / 2.0)
            }
            GearKind::Worm => Some((self.worm_d + self.module * f64::from(self.mate)) / 2.0),
            _ => None,
        }
    }

    /// A worm's lead angle, degrees.
    fn lead_angle(&self) -> f64 {
        let lead = f64::from(self.starts) * PI * self.module;
        (lead / (PI * self.worm_d)).atan().to_degrees()
    }

    /// A bevel gear's pitch cone: its half angle (radians), its cone
    /// distance, and its face width as built.
    fn cone(&self) -> (f64, f64, f64) {
        let delta = (f64::from(self.teeth) / f64::from(self.mate.max(1))).atan();
        let a = self.pitch_radius() / delta.sin();
        (delta, a, self.thickness.min(a / 3.0))
    }

    /// Where the body ends up the axis, under any hub.
    fn body_top(&self) -> f64 {
        match self.kind {
            GearKind::Worm => self.length,
            GearKind::Bevel => self.cone().2 * self.cone().0.cos(),
            GearKind::Pulley => self.thickness + if self.flanges { 2.0 } else { 0.0 },
            _ => self.thickness,
        }
    }

    /// The height of the whole part up the axis.
    fn height(&self) -> f64 {
        self.body_top() + if self.hub > 0.0 { self.hub_h } else { 0.0 }
    }

    /// A tooth count's involute outline about the origin: `z` teeth of
    /// module `m`, flanks at `alpha` (radians) out to `addendum` over the
    /// pitch circle and in to `dedendum` under it, each tooth thinned by
    /// `backlash` at the pitch line, the first tooth centred `phase`
    /// (radians) round from +X.
    #[allow(clippy::too_many_arguments)]
    fn involute(
        z: u32,
        m: f64,
        alpha: f64,
        addendum: f64,
        dedendum: f64,
        backlash: f64,
        phase: f64,
        steps: usize,
    ) -> Vec<[f64; 2]> {
        let zf = f64::from(z);
        let r = m * zf / 2.0;
        let base = r * alpha.cos();
        let tip = r + addendum;
        let root = (r - dedendum).max(0.3 * r);
        let inv = |radius: f64| {
            let a = (base / radius).min(1.0).acos();
            a.tan() - a
        };
        let half = PI / (2.0 * zf) - backlash / (2.0 * r) + inv(r);
        let start = base.max(root);
        let mut points = Vec::with_capacity(z as usize * (2 * steps + 4));
        for i in 0..z {
            let centre = phase + TAU * f64::from(i) / zf;
            let at = |radius: f64, angle: f64| [radius * angle.cos(), radius * angle.sin()];
            if root < start {
                points.push(at(root, centre - (half - inv(start))));
            }
            for s in 0..=steps {
                let radius = start + (tip - start) * s as f64 / steps as f64;
                points.push(at(radius, centre - (half - inv(radius))));
            }
            for s in (0..=steps).rev() {
                let radius = start + (tip - start) * s as f64 / steps as f64;
                points.push(at(radius, centre + (half - inv(radius))));
            }
            if root < start {
                points.push(at(root, centre + (half - inv(start))));
            }
        }
        points
    }

    /// The gear's outline across, in `(x, y)`: teeth out, or a pulley's
    /// grooves in; a ring gear's is the hole in its rim.
    pub fn outline(&self) -> Vec<[f64; 2]> {
        let alpha = self.pressure.to_radians();
        let m = self.module;
        match self.kind {
            GearKind::Internal => {
                // The spaces of a ring are the teeth of the gear it runs
                // on: that gear's outline, out to the ring's roots, turned
                // half a pitch.
                Self::involute(
                    self.teeth,
                    m,
                    alpha,
                    1.25 * m,
                    m,
                    -self.backlash,
                    PI / f64::from(self.teeth),
                    6,
                )
            }
            GearKind::Pulley => {
                // Round grooves in the tip circle, as the belt's teeth are.
                let (ro, z) = (self.outer_radius(), self.teeth);
                let rg = GT2_DEPTH * 0.72;
                let rc = ro - GT2_DEPTH + rg;
                let mut pts = Vec::new();
                for i in 0..z {
                    let a = TAU * f64::from(i) / f64::from(z);
                    let c = [rc * a.cos(), rc * a.sin()];
                    // The groove, in from its right edge round to its left.
                    let half = (rg / rc).asin();
                    let edge = |t: f64| [ro * t.cos(), ro * t.sin()];
                    let e0 = edge(a - half);
                    let b0 = (e0[1] - c[1]).atan2(e0[0] - c[0]);
                    let e1 = edge(a + half);
                    let mut b1 = (e1[1] - c[1]).atan2(e1[0] - c[0]);
                    while b1 > b0 {
                        b1 -= TAU;
                    }
                    pts.extend(geom::arc_points(c, rg, b0, b1, 6));
                    // The tip between this groove and the next.
                    let next = TAU * f64::from(i + 1) / f64::from(z) - half;
                    let mut tip = geom::arc_points([0.0, 0.0], ro, a + half, next, 3);
                    tip.pop();
                    pts.extend(tip);
                }
                pts
            }
            _ => Self::involute(
                self.teeth,
                m,
                alpha,
                m,
                1.25 * m,
                self.backlash,
                0.0,
                if self.kind.is_wheel() && self.kind != GearKind::Spur {
                    4
                } else {
                    6
                },
            ),
        }
    }

    /// The shaft hole's outline, in `(x, y)`, or a circle.
    fn shaft_wire(&self) -> Vec<ProfileSegment> {
        let r = self.bore / 2.0;
        match self.shaft {
            Shaft::Round => vec![geom::circle(self.bore)],
            Shaft::Hex => geom::polygon(&geom::hexagon(self.bore)),
            Shaft::D => {
                let f = (self.flat - r).clamp(-r + 0.1, r - 0.05);
                let phi = (f / r).asin();
                geom::polygon(&geom::arc_points([0.0, 0.0], r, -phi, PI + phi, 40))
            }
            Shaft::DoubleD => {
                let f = (self.flat / 2.0).clamp(0.1, r - 0.05);
                let phi = (f / r).asin();
                let mut pts = geom::arc_points([0.0, 0.0], r, -phi, phi, 20);
                pts.extend(geom::arc_points([0.0, 0.0], r, PI - phi, PI + phi, 20));
                geom::polygon(&pts)
            }
            Shaft::Keyed => {
                let w = (self.key_w / 2.0).clamp(0.1, r * 0.9);
                let psi = (w / r).asin();
                let mut pts =
                    geom::arc_points([0.0, 0.0], r, PI / 2.0 + psi, TAU + PI / 2.0 - psi, 40);
                let top = r + self.key_d.max(0.1);
                pts.push([w, top]);
                pts.push([-w, top]);
                geom::polygon(&pts)
            }
        }
    }

    /// How far out from the axis the bore reaches.
    fn bore_reach(&self) -> f64 {
        match self.shaft {
            Shaft::Hex => self.bore / (PI / 6.0).cos() / 2.0,
            Shaft::Keyed => self.bore / 2.0 + self.key_d,
            _ => self.bore / 2.0,
        }
    }

    /// Points rotated by `angle` about the origin.
    fn turned(points: &[[f64; 2]], angle: f64) -> Vec<[f64; 2]> {
        let (s, c) = angle.sin_cos();
        points
            .iter()
            .map(|p| [p[0] * c - p[1] * s, p[0] * s + p[1] * c])
            .collect()
    }

    fn section(points: &[[f64; 2]], z: f64) -> Profile {
        Profile {
            plane: geom::xy(z),
            wires: vec![ProfileWire::new(geom::polygon(points))],
        }
    }

    /// A ruled loft through the outline turned `twist` radians over
    /// `height`, in enough sections to keep each turn small.
    fn loft(outline: &[[f64; 2]], z0: f64, height: f64, twist: f64) -> SolidOp {
        let n = ((twist.abs().to_degrees() / 10.0).ceil() as usize).clamp(1, 12);
        let sections = (0..=n)
            .map(|i| {
                let t = i as f64 / n as f64;
                Self::section(&Self::turned(outline, twist * t), z0 + height * t)
            })
            .collect();
        SolidOp::Loft {
            sections,
            ruled: true,
            closed: false,
            op: BooleanOp::NewSolid,
        }
    }

    /// The twist across `height` of a helical gear: the helix angle at
    /// the pitch circle.
    fn twist(&self, height: f64) -> f64 {
        let t = height * self.helix.to_radians().tan() / self.pitch_radius();
        if self.left { -t } else { t }
    }

    /// A rack's outline in `(x, z)`: trapezoid teeth along X on a back,
    /// flanks at the pressure angle.
    fn rack_outline(&self) -> Vec<[f64; 2]> {
        let (m, l) = (self.module, self.length);
        let p = PI * m;
        let n = ((l / p).floor() as usize).max(1);
        let back = (2.0 * m).max(3.0);
        let (root, tip) = (back, back + 2.25 * m);
        let tan = self.pressure.to_radians().tan();
        let half_pitch = (p / 2.0 - self.backlash) / 2.0;
        let x0 = -p * n as f64 / 2.0;
        let mut pts = vec![[-l / 2.0, 0.0], [l / 2.0, 0.0], [l / 2.0, root]];
        for i in (0..n).rev() {
            let c = x0 + p * (i as f64 + 0.5);
            let at_root = half_pitch + 1.25 * m * tan;
            let at_tip = half_pitch - m * tan;
            pts.push([c + at_root, root]);
            pts.push([c + at_tip, tip]);
            pts.push([c - at_tip, tip]);
            pts.push([c - at_root, root]);
        }
        pts.push([-l / 2.0, root]);
        pts
    }

    /// The body, before the bore, the hub and the set screw.
    fn body_ops(&self) -> Vec<SolidOp> {
        let t = self.thickness;
        match self.kind {
            GearKind::Spur => {
                vec![geom::prism(
                    &self.outline(),
                    Vec::new(),
                    0.0,
                    t,
                    BooleanOp::NewSolid,
                )]
            }
            GearKind::Helical => vec![Self::loft(&self.outline(), 0.0, t, self.twist(t))],
            GearKind::Herringbone => {
                // Out one way to the middle and back the other: one ruled
                // loft through three sections.
                let outline = self.outline();
                let twist = self.twist(t / 2.0);
                let sections = vec![
                    Self::section(&outline, 0.0),
                    Self::section(&Self::turned(&outline, twist), t / 2.0),
                    Self::section(&outline, t),
                ];
                vec![SolidOp::Loft {
                    sections,
                    ruled: true,
                    closed: false,
                    op: BooleanOp::NewSolid,
                }]
            }
            GearKind::Internal => vec![
                geom::prism(
                    &geom::regular(72, 2.0 * self.outer_radius() * (PI / 72.0).cos()),
                    Vec::new(),
                    0.0,
                    t,
                    BooleanOp::NewSolid,
                ),
                geom::extrude(
                    geom::xy(-1.0),
                    vec![geom::polygon(&self.outline())],
                    t + 2.0,
                    BooleanOp::Cut,
                ),
            ],
            GearKind::Rack => vec![geom::extrude(
                ProfilePlane {
                    origin: [0.0, t / 2.0, 0.0],
                    x_axis: [1.0, 0.0, 0.0],
                    y_axis: [0.0, 0.0, 1.0],
                    normal: [0.0, -1.0, 0.0],
                },
                vec![geom::polygon(&self.rack_outline())],
                t,
                BooleanOp::NewSolid,
            )],
            GearKind::Worm => {
                // A cylinder at the tip diameter with the thread's groove
                // cut round it: an axial pitch of the module, a lead of
                // the starts, flanks at the pressure angle.
                let (ro, l, m) = (self.outer_radius(), self.length, self.module);
                let c = (0.5 * m).min(ro / 4.0);
                let mut ops = vec![geom::revolve(
                    &[
                        [0.0, 0.0],
                        [ro - c, 0.0],
                        [ro, c],
                        [ro, l - c],
                        [ro - c, l],
                        [0.0, l],
                    ],
                    BooleanOp::NewSolid,
                )];
                let p = PI * m;
                let lead = p * f64::from(self.starts);
                let tan = self.pressure.to_radians().tan();
                // The groove's flat at the root: half a pitch at the pitch
                // line, less the flanks' lean down to the root.
                let flat = (p / 2.0 - self.backlash - 2.0 * 1.25 * m * tan).max(0.1 * m);
                for i in 0..self.starts {
                    ops.push(geom::thread_groove_at(
                        ro,
                        self.root_radius(),
                        flat,
                        self.pressure,
                        lead,
                        l,
                        l,
                        false,
                        self.left,
                        360.0 * f64::from(i) / f64::from(self.starts),
                    ));
                }
                ops
            }
            GearKind::Bevel => {
                // A straight bevel, near enough: the outline lofted to a
                // smaller copy of itself up the pitch cone.
                let (delta, a, f) = self.cone();
                let scale = 1.0 - f / a;
                let outline = self.outline();
                let small: Vec<[f64; 2]> = outline
                    .iter()
                    .map(|p| [p[0] * scale, p[1] * scale])
                    .collect();
                vec![SolidOp::Loft {
                    sections: vec![
                        Self::section(&outline, 0.0),
                        Self::section(&small, f * delta.cos()),
                    ],
                    ruled: true,
                    closed: false,
                    op: BooleanOp::NewSolid,
                }]
            }
            GearKind::Pulley => {
                let z0 = if self.flanges { 1.0 } else { 0.0 };
                let mut ops = vec![geom::prism(
                    &self.outline(),
                    Vec::new(),
                    z0,
                    t,
                    BooleanOp::NewSolid,
                )];
                if self.flanges {
                    let fd = 2.0 * self.outer_radius() + 3.0;
                    let flange = geom::regular(72, fd * (PI / 72.0).cos());
                    ops.push(geom::prism(&flange, Vec::new(), 0.0, 1.05, BooleanOp::Fuse));
                    ops.push(geom::prism(
                        &flange,
                        Vec::new(),
                        z0 + t - 0.05,
                        1.05,
                        BooleanOp::Fuse,
                    ));
                }
                ops
            }
        }
    }

    /// Where the set screw goes in: the hub, or the body when there is
    /// none; its height up the axis and the radius it comes in from.
    fn set_screw_at(&self) -> Option<(f64, f64)> {
        if self.set_screw <= 0.0 || !self.kind.has_bore() {
            return None;
        }
        if self.hub > 0.0 {
            Some((self.body_top() + self.hub_h / 2.0, self.hub / 2.0))
        } else {
            let z = match self.kind {
                GearKind::Worm => self.length / 2.0,
                _ => self.thickness / 2.0,
            };
            Some((z, self.root_radius()))
        }
    }

    /// The mesh notes under the drawing: what it runs with and where.
    fn mesh_text(&self) -> String {
        let r2 = |v: f64| fmt((v * 100.0).round() / 100.0);
        match self.kind {
            GearKind::Spur | GearKind::Helical | GearKind::Herringbone => {
                let hand = match self.kind {
                    GearKind::Spur => String::new(),
                    _ => format!(
                        " (the mate {}-handed{})",
                        if self.left { "right" } else { "left" },
                        if self.kind == GearKind::Herringbone {
                            " too"
                        } else {
                            ""
                        }
                    ),
                };
                format!(
                    "Pitch Ø{} · outside Ø{} · centre distance {} to a {}-tooth mate{hand}",
                    r2(2.0 * self.pitch_radius()),
                    r2(2.0 * self.outer_radius()),
                    r2(self.centre_distance().unwrap_or(0.0)),
                    self.mate
                )
            }
            GearKind::Internal => format!(
                "Pitch Ø{} · teeth tips at Ø{} · outside Ø{} · takes a module {} gear",
                r2(2.0 * self.pitch_radius()),
                r2(2.0 * self.root_radius()),
                r2(2.0 * self.outer_radius()),
                fmt(self.module)
            ),
            GearKind::Rack => format!(
                "Pitch {} · pitch line {} over the back · runs with any module {} gear",
                r2(PI * self.module),
                r2((2.0 * self.module).max(3.0) + 1.25 * self.module),
                fmt(self.module)
            ),
            GearKind::Worm => format!(
                "Worm Ø{} over the thread, lead angle {}° · wheel: a {}-tooth helical gear, module {}, at {}° {}-hand · centre distance {}",
                r2(2.0 * self.outer_radius()),
                r2(self.lead_angle()),
                self.mate,
                fmt(self.module),
                r2(self.lead_angle()),
                if self.left { "left" } else { "right" },
                r2(self.centre_distance().unwrap_or(0.0))
            ),
            GearKind::Bevel => {
                let (delta, a, f) = self.cone();
                format!(
                    "Pitch Ø{} · pitch cone {}° · cone distance {} · face {} · a {}-tooth mate at 90°",
                    r2(2.0 * self.pitch_radius()),
                    r2(delta.to_degrees()),
                    r2(a),
                    r2(f),
                    self.mate
                )
            }
            GearKind::Pulley => format!(
                "GT2 belt · pitch Ø{} · outside Ø{} · belts up to {} wide",
                r2(2.0 * self.pitch_radius()),
                r2(2.0 * self.outer_radius()),
                fmt((self.thickness - 1.0).max(1.0))
            ),
        }
    }

    /// Which maker's shaft the bore is, by its index in `SHAFTS`.
    fn preset(&self) -> usize {
        SHAFTS
            .iter()
            .position(|(_, bore, shaft, flat)| {
                *bore > 0.0
                    && (bore - self.bore).abs() < 1e-9
                    && *shaft == self.shaft
                    && (*flat == 0.0 || (flat - self.flat).abs() < 1e-9)
            })
            .unwrap_or(0)
    }
}

impl Part for Gear {
    const FAMILY: Family = Family::Gear;

    fn label(&self) -> String {
        match self.kind {
            GearKind::Rack => format!("m{} rack × {}", fmt(self.module), fmt(self.length)),
            GearKind::Worm => format!(
                "m{} worm Ø{} × {}{}",
                fmt(self.module),
                fmt(self.worm_d),
                fmt(self.length),
                if self.starts > 1 {
                    format!(", {}-start", self.starts)
                } else {
                    String::new()
                }
            ),
            GearKind::Pulley => format!("GT2 {}T pulley", self.teeth),
            _ => format!(
                "m{} {}T {}",
                fmt(self.module),
                self.teeth,
                self.kind.short()
            ),
        }
    }

    fn icon(&self) -> &'static str {
        self.kind.icon()
    }

    fn problem(&self) -> Option<String> {
        if self.teeth < 6 && self.kind != GearKind::Worm {
            return Some("A gear needs at least 6 teeth.".into());
        }
        if self.kind != GearKind::Pulley && self.module <= 0.0 {
            return Some("The module must be more than 0.".into());
        }
        if self.thickness <= 0.0 {
            return Some("The face width must be more than 0.".into());
        }
        if !(10.0..=35.0).contains(&self.pressure) {
            return Some("The pressure angle must lie between 10° and 35°.".into());
        }
        if self.backlash < 0.0 || self.backlash >= PI * self.module / 4.0 {
            return Some("The backlash must be less than a quarter of the circular pitch.".into());
        }
        if matches!(self.kind, GearKind::Helical | GearKind::Herringbone)
            && !(1.0..=45.0).contains(&self.helix)
        {
            return Some("The helix angle must lie between 1° and 45°.".into());
        }
        if self.kind.has_bore() {
            if self.bore < 0.0 {
                return Some("The bore must be 0 or more.".into());
            }
            let room = match self.kind {
                GearKind::Bevel => self.root_radius() * (1.0 - self.cone().2 / self.cone().1),
                _ => self.root_radius(),
            };
            if self.bore > 0.0 && self.bore_reach() >= room - 0.6 {
                return Some("The bore must fit inside the teeth's roots with a wall.".into());
            }
            if self.bore > 0.0
                && self.shaft == Shaft::D
                && (self.flat <= self.bore / 2.0 || self.flat >= self.bore)
            {
                return Some(
                    "A D shaft's flat must lie between half the bore and the bore.".into(),
                );
            }
            if self.bore > 0.0
                && self.shaft == Shaft::DoubleD
                && (self.flat <= 0.0 || self.flat >= self.bore)
            {
                return Some("A double D's flats must stand less than the bore apart.".into());
            }
            if self.bore > 0.0
                && self.shaft == Shaft::Keyed
                && (self.key_w <= 0.0 || self.key_w >= self.bore || self.key_d <= 0.0)
            {
                return Some("A keyway must be narrower than the bore and deeper than 0.".into());
            }
        }
        if self.hub > 0.0 {
            if self.hub_h <= 0.0 {
                return Some("The hub must be taller than 0.".into());
            }
            if self.kind.has_bore() && self.hub / 2.0 <= self.bore_reach() + 0.6 {
                return Some("The hub must be wider than the bore with a wall.".into());
            }
            if self.kind.is_wheel() && self.hub / 2.0 >= self.root_radius() {
                return Some("The hub must be narrower than the teeth's roots.".into());
            }
        }
        if let Some((_, reach)) = self.set_screw_at() {
            if reach - self.bore_reach() < self.set_screw + 0.5 {
                return Some(
                    "Too little metal for the set screw: add a hub or make it wider.".into(),
                );
            }
            let room = if self.hub > 0.0 {
                self.hub_h
            } else {
                self.thickness
            };
            if room < self.set_screw + 1.0 {
                return Some("The hub is too short for the set screw.".into());
            }
        }
        if self.kind == GearKind::Rack && self.length < PI * self.module {
            return Some("The rack is too short for a tooth.".into());
        }
        if self.kind == GearKind::Worm {
            if self.length <= 0.0 || self.worm_d <= 0.0 {
                return Some("The worm needs a diameter and a length.".into());
            }
            if self.root_radius() <= 0.5 {
                return Some("The worm is too thin for its module.".into());
            }
            if self.starts == 0 || self.starts > 4 {
                return Some("A worm has 1 to 4 starts.".into());
            }
        }
        if self.kind == GearKind::Internal && self.rim <= 0.0 {
            return Some("The rim must be thicker than 0.".into());
        }
        if self.kind == GearKind::Bevel && self.mate < 6 {
            return Some("A bevel gear's mate needs at least 6 teeth.".into());
        }
        None
    }

    fn axis(&self) -> ([f64; 3], [f64; 3]) {
        if self.kind == GearKind::Rack {
            (
                [-self.length / 2.0, 0.0, 0.0],
                [self.length / 2.0, 0.0, 0.0],
            )
        } else {
            super::z_axis(0.0, self.height())
        }
    }

    fn ops(&self) -> Vec<SolidOp> {
        let mut ops = self.body_ops();
        let top = self.body_top();
        if self.hub > 0.0 && self.kind.has_bore() {
            ops.push(geom::prism(
                &geom::regular(48, self.hub * (PI / 48.0).cos()),
                Vec::new(),
                top - 0.05,
                self.hub_h + 0.05,
                BooleanOp::Fuse,
            ));
        }
        if self.kind.has_bore() && self.bore > 0.0 {
            let h = self.height();
            ops.push(geom::extrude(
                geom::xy(-1.0),
                vec![self.shaft_wire()],
                h + 2.0,
                BooleanOp::Cut,
            ));
        }
        if let Some((z, reach)) = self.set_screw_at() {
            // The set screw's tapped hole in from +X, at the tap drill.
            let d = self.set_screw - 1.0825 * crate::standards::coarse_pitch(self.set_screw);
            let x0 = self.bore_reach().max(0.0) - 0.5;
            ops.push(geom::extrude(
                ProfilePlane {
                    origin: [x0, 0.0, z],
                    x_axis: [0.0, 1.0, 0.0],
                    y_axis: [0.0, 0.0, 1.0],
                    normal: [1.0, 0.0, 0.0],
                },
                vec![vec![geom::circle(d)]],
                reach - x0 + 1.0,
                BooleanOp::Cut,
            ));
        }
        ops
    }

    fn panel(&self, ctx: &Ctx) -> Vec<Widget> {
        let kinds: Vec<&str> = GearKind::ALL.iter().map(|k| k.name()).collect();
        let k = self.kind;
        let mut widgets = vec![
            self.drawing(ctx),
            choice(
                "kind",
                "Gear",
                &kinds,
                GearKind::ALL.iter().position(|x| *x == k).unwrap_or(0),
            ),
        ];
        // The teeth.
        let mut teeth = Vec::new();
        if k != GearKind::Worm {
            teeth.push(count(ctx, "teeth", "Teeth", f64::from(self.teeth), 6.0));
        }
        if k != GearKind::Pulley {
            teeth.push(number(ctx, "module", "Module", self.module, 0.1, 2));
            teeth.push(number(
                ctx,
                "pressure",
                "Pressure angle",
                self.pressure,
                10.0,
                1,
            ));
        }
        let width = match k {
            GearKind::Rack | GearKind::Bevel => "Face width",
            GearKind::Pulley => "Toothed width",
            _ => "Thickness",
        };
        if k != GearKind::Worm {
            teeth.push(number(ctx, "thickness", width, self.thickness, 0.1, 2));
        }
        if matches!(k, GearKind::Rack | GearKind::Worm) {
            teeth.push(number(ctx, "length", "Length", self.length, 0.1, 1));
        }
        if k == GearKind::Worm {
            teeth.push(number(ctx, "worm_d", "Pitch diameter", self.worm_d, 0.1, 2));
            teeth.push(count(ctx, "starts", "Starts", f64::from(self.starts), 1.0));
        }
        if k == GearKind::Internal {
            teeth.push(number(
                ctx,
                "rim",
                "Rim outside the teeth",
                self.rim,
                0.1,
                2,
            ));
        }
        if matches!(k, GearKind::Helical | GearKind::Herringbone) {
            teeth.push(number(ctx, "helix", "Helix angle", self.helix, 1.0, 1));
        }
        if matches!(
            k,
            GearKind::Helical | GearKind::Herringbone | GearKind::Worm
        ) {
            teeth.push(choice(
                "hand",
                "Hand",
                &["Right", "Left"],
                usize::from(self.left),
            ));
        }
        if k != GearKind::Pulley {
            teeth.push(number(ctx, "backlash", "Backlash", self.backlash, 0.0, 2));
        }
        if matches!(
            k,
            GearKind::Spur
                | GearKind::Helical
                | GearKind::Herringbone
                | GearKind::Worm
                | GearKind::Bevel
        ) {
            let label = if k == GearKind::Worm {
                "Wheel teeth"
            } else {
                "Mate's teeth"
            };
            teeth.push(count(ctx, "mate", label, f64::from(self.mate), 6.0));
        }
        if k == GearKind::Pulley {
            teeth.push(toggle("flanges", "Flanges", self.flanges));
        }
        teeth.push(super::text(self.mesh_text()));
        widgets.push(group("Teeth", true, teeth));
        // The shaft and the hub.
        if k.has_bore() {
            let presets: Vec<&str> = SHAFTS.iter().map(|s| s.0).collect();
            let shafts: Vec<&str> = Shaft::ALL.iter().map(|s| s.name()).collect();
            let bore_label = if self.shaft == Shaft::Hex {
                "Across flats (0: none)"
            } else {
                "Bore (0: none)"
            };
            let mut shaft = vec![
                choice("preset", "Shaft", &presets, self.preset()),
                number(ctx, "bore", bore_label, self.bore, 0.0, 2),
                choice(
                    "shaft",
                    "Form",
                    &shafts,
                    Shaft::ALL
                        .iter()
                        .position(|s| *s == self.shaft)
                        .unwrap_or(0),
                ),
            ];
            match self.shaft {
                Shaft::D => shaft.push(number(ctx, "flat", "Across the flat", self.flat, 0.1, 2)),
                Shaft::DoubleD => {
                    shaft.push(number(ctx, "flat", "Across the flats", self.flat, 0.1, 2))
                }
                Shaft::Keyed => {
                    shaft.push(number(ctx, "key_w", "Keyway width", self.key_w, 0.1, 2));
                    shaft.push(number(ctx, "key_d", "Keyway depth", self.key_d, 0.1, 2));
                }
                _ => {}
            }
            widgets.push(group("Shaft", true, shaft));
            let mut hub = vec![number(
                ctx,
                "hub",
                "Hub diameter (0: none)",
                self.hub,
                0.0,
                2,
            )];
            if self.hub > 0.0 {
                hub.push(number(ctx, "hub_h", "Hub height", self.hub_h, 0.1, 2));
            }
            let screws = ["None", "M3", "M4", "M5"];
            let selected = match self.set_screw.round() as u32 {
                3 => 1,
                4 => 2,
                5 => 3,
                _ => 0,
            };
            hub.push(choice("set_screw", "Set screw", &screws, selected));
            widgets.push(group("Hub", self.hub > 0.0 || self.set_screw > 0.0, hub));
        }
        widgets.extend(note(self.problem()));
        widgets
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            integer("teeth", "Teeth"),
            length("module", "Module"),
            length("thickness", "Thickness"),
            length("backlash", "Backlash"),
            length("bore", "Bore"),
            length("flat", "Across the flat"),
            length("key_w", "Keyway width"),
            length("key_d", "Keyway depth"),
            length("hub", "Hub diameter"),
            length("hub_h", "Hub height"),
            length("length", "Length"),
            length("rim", "Rim"),
            length("worm_d", "Worm pitch diameter"),
            integer("mate", "Mate's teeth"),
            integer("starts", "Starts"),
        ]
    }

    fn apply(&mut self, event: &PanelEvent) -> bool {
        match event {
            PanelEvent::Choice { id, index } => match id.as_str() {
                "kind" => {
                    self.kind = GearKind::ALL.get(*index).copied().unwrap_or(GearKind::Spur);
                    self.suit();
                }
                "hand" => self.left = *index == 1,
                "preset" => {
                    if let Some((_, bore, shaft, flat)) = SHAFTS.get(*index)
                        && *bore > 0.0
                    {
                        self.bore = *bore;
                        self.shaft = *shaft;
                        if *flat > 0.0 {
                            self.flat = *flat;
                        }
                    }
                }
                "shaft" => {
                    self.shaft = Shaft::ALL.get(*index).copied().unwrap_or(Shaft::Round);
                    if self.shaft == Shaft::D
                        && (self.flat <= self.bore / 2.0 || self.flat >= self.bore)
                    {
                        self.flat = ((0.9 * self.bore) * 10.0).round() / 10.0;
                    }
                    if self.shaft == Shaft::DoubleD && (self.flat <= 0.0 || self.flat >= self.bore)
                    {
                        self.flat = ((0.7 * self.bore) * 10.0).round() / 10.0;
                    }
                }
                "set_screw" => self.set_screw = [0.0, 3.0, 4.0, 5.0][(*index).min(3)],
                _ => return false,
            },
            PanelEvent::Number { id, value } => match id.as_str() {
                "teeth" => self.teeth = value.round().max(1.0) as u32,
                "module" => self.module = *value,
                "pressure" => self.pressure = *value,
                "thickness" => self.thickness = *value,
                "backlash" => self.backlash = *value,
                "helix" => self.helix = *value,
                "mate" => self.mate = value.round().max(1.0) as u32,
                "bore" => self.bore = *value,
                "flat" => self.flat = *value,
                "key_w" => self.key_w = *value,
                "key_d" => self.key_d = *value,
                "hub" => self.hub = *value,
                "hub_h" => self.hub_h = *value,
                "length" => self.length = *value,
                "rim" => self.rim = *value,
                "worm_d" => self.worm_d = *value,
                "starts" => self.starts = value.round().max(1.0) as u32,
                _ => return false,
            },
            PanelEvent::Toggle { id, on } if id == "flanges" => self.flanges = *on,
            _ => return false,
        }
        true
    }

    fn with_args(args: &Value, _defaults: &Defaults) -> Result<Self, String> {
        let kind = match arg_str(args, "kind") {
            None => GearKind::Spur,
            Some(name) => GearKind::named(name).ok_or_else(|| {
                format!(
                    "`{name}` is not a gear: {}",
                    GearKind::ALL.map(|k| k.tool()).join(", ")
                )
            })?,
        };
        let mut gear = Gear::new(kind);
        if let Some(name) = arg_str(args, "shaft") {
            gear.shaft = Shaft::ALL
                .into_iter()
                .find(|s| {
                    s.tool().eq_ignore_ascii_case(name) || s.name().eq_ignore_ascii_case(name)
                })
                .ok_or_else(|| format!("`{name}` is not a shaft form: round, d, dd, hex, keyed"))?;
        }
        if let Some(name) = arg_str(args, "hand") {
            gear.left = name.eq_ignore_ascii_case("left");
        }
        for (key, slot) in [
            ("module", &mut gear.module),
            ("pressure", &mut gear.pressure),
            ("thickness", &mut gear.thickness),
            ("backlash", &mut gear.backlash),
            ("helix", &mut gear.helix),
            ("bore", &mut gear.bore),
            ("flat", &mut gear.flat),
            ("key_w", &mut gear.key_w),
            ("key_d", &mut gear.key_d),
            ("hub", &mut gear.hub),
            ("hub_h", &mut gear.hub_h),
            ("set_screw", &mut gear.set_screw),
            ("length", &mut gear.length),
            ("rim", &mut gear.rim),
            ("worm_d", &mut gear.worm_d),
        ] {
            if let Some(v) = arg_f64(args, key) {
                *slot = v;
            }
        }
        for (key, slot) in [
            ("teeth", &mut gear.teeth),
            ("mate", &mut gear.mate),
            ("starts", &mut gear.starts),
        ] {
            if let Some(v) = arg_f64(args, key) {
                *slot = v.round().max(1.0) as u32;
            }
        }
        if let Some(on) = arg_bool(args, "flanges") {
            gear.flanges = on;
        }
        Ok(gear)
    }
}

impl Gear {
    /// The face: the teeth round, the shaft hole, the hub as a ring; a
    /// rack or a worm from the side.
    fn drawing(&self, ctx: &Ctx) -> Widget {
        let mut s = Sketch::new(ctx.focus);
        match self.kind {
            GearKind::Rack => {
                let (m, l, t) = (self.module, self.length, self.thickness);
                let back = (2.0 * m).max(3.0);
                let off = Sketch::standoff(l * 0.5);
                s.poly(&self.rack_outline(), DiagramStroke::Outline, false);
                s.line(
                    &[[-l / 2.0, back + 1.25 * m], [l / 2.0, back + 1.25 * m]],
                    DiagramStroke::Axis,
                );
                s.width(
                    "length",
                    -l / 2.0,
                    l / 2.0,
                    0.0,
                    -off,
                    format!("L {}", fmt(l)),
                );
                s.height(
                    "module",
                    l / 2.0,
                    back,
                    back + 2.25 * m,
                    -off * 0.5,
                    format!("m {}", fmt(m)),
                );
                s.callout(
                    "thickness",
                    [0.0, back / 2.0],
                    [l * 0.25, -off * 2.2],
                    format!("face {}", fmt(t)),
                );
            }
            GearKind::Worm => {
                let (ro, ri, l) = (self.outer_radius(), self.root_radius(), self.length);
                let off = Sketch::standoff(l.max(2.0 * ro) * 0.6);
                s.rect([-ro, 0.0], [ro, l]);
                let p = PI * self.module * f64::from(self.starts);
                let mut z = p * 0.3;
                while z < l - 0.2 {
                    let lean = if self.left { -p * 0.25 } else { p * 0.25 };
                    s.line(
                        &[[-ro, z], [ro, (z + lean).clamp(0.0, l)]],
                        DiagramStroke::Thin,
                    );
                    z += p;
                }
                s.hidden(&[[-ri, 0.0], [-ri, l]]);
                s.hidden(&[[ri, 0.0], [ri, l]]);
                if self.bore > 0.0 {
                    let b = self.bore / 2.0;
                    s.hidden(&[[-b, 0.0], [-b, l]]);
                    s.hidden(&[[b, 0.0], [b, l]]);
                    s.width("bore", -b, b, 0.0, -off, format!("Ø{}", fmt(self.bore)));
                }
                s.width(
                    "worm_d",
                    -ro,
                    ro,
                    l,
                    off,
                    format!("Ø{} over the thread", fmt(2.0 * ro)),
                );
                s.height("length", -ro, 0.0, l, off, format!("L {}", fmt(l)));
                s.axis(0.0, -off * 0.4, l + off * 0.4);
            }
            _ => {
                let (ro, rp) = (self.outer_radius(), self.pitch_radius());
                let off = Sketch::standoff(2.0 * ro);
                let outline = self.outline();
                if self.kind == GearKind::Internal {
                    s.circle([0.0, 0.0], 2.0 * ro, DiagramStroke::Outline, false);
                    s.poly(&outline, DiagramStroke::Outline, false);
                    s.circle([0.0, 0.0], 2.0 * rp, DiagramStroke::Axis, false);
                    s.width(
                        "rim",
                        ro - self.rim,
                        ro,
                        0.0,
                        0.0,
                        format!("rim {}", fmt(self.rim)),
                    );
                } else {
                    s.poly(&outline, DiagramStroke::Outline, false);
                    s.circle([0.0, 0.0], 2.0 * rp, DiagramStroke::Axis, false);
                }
                if self.kind.has_bore() && self.bore > 0.0 {
                    let pts: Vec<[f64; 2]> = match self.shaft {
                        Shaft::Round => geom::arc_points([0.0, 0.0], self.bore / 2.0, 0.0, TAU, 40),
                        _ => self
                            .shaft_wire()
                            .iter()
                            .filter_map(|seg| match seg {
                                ProfileSegment::Line { start, .. } => Some(*start),
                                _ => None,
                            })
                            .collect(),
                    };
                    s.poly(&pts, DiagramStroke::Outline, false);
                    s.width(
                        "bore",
                        -self.bore / 2.0,
                        self.bore / 2.0,
                        0.0,
                        0.0,
                        format!("Ø{}", fmt(self.bore)),
                    );
                }
                if self.hub > 0.0 && self.kind.has_bore() {
                    s.circle([0.0, 0.0], self.hub, DiagramStroke::Hidden, false);
                    s.width(
                        "hub",
                        -self.hub / 2.0,
                        self.hub / 2.0,
                        -self.hub / 2.0,
                        -off * 0.5,
                        format!("hub Ø{}", fmt(self.hub)),
                    );
                }
                let d_label = match self.kind {
                    GearKind::Internal => "outside Ø",
                    _ => "tip Ø",
                };
                s.width(
                    "teeth",
                    -ro,
                    ro,
                    ro,
                    off,
                    format!(
                        "{d_label}{} · {}T",
                        fmt((2.0 * ro * 100.0).round() / 100.0),
                        self.teeth
                    ),
                );
                let (key, text) = if self.kind == GearKind::Pulley {
                    ("thickness", format!("GT2 · {} wide", fmt(self.thickness)))
                } else {
                    (
                        "module",
                        format!("m {} · {} thick", fmt(self.module), fmt(self.thickness)),
                    )
                };
                s.callout(
                    key,
                    [ro * 0.7, -ro * 0.7],
                    [ro + off * 0.5, -ro - off * 0.8],
                    text,
                );
            }
        }
        s.finish("gear")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printcad_bench_sdk::json;

    #[test]
    fn a_spur_gear_is_a_prism_of_its_involute_with_a_d_bore() {
        let g = Gear::new(GearKind::Spur);
        assert_eq!(
            (g.teeth, g.module, g.bore, g.shaft, g.flat),
            (20, 2.0, 5.0, Shaft::D, 4.5)
        );
        assert_eq!(g.label(), "m2 20T spur gear");
        assert_eq!(g.problem(), None);
        assert_eq!((g.pitch_radius(), g.outer_radius()), (20.0, 22.0));
        assert_eq!(g.centre_distance(), Some(40.0));
        let roles: Vec<_> = g.ops().iter().map(|op| op.boolean_op()).collect();
        assert_eq!(roles, [Some(BooleanOp::NewSolid), Some(BooleanOp::Cut)]);
        let outline = g.outline();
        assert_eq!(outline.len(), 20 * 16);
        let far = outline.iter().map(|p| p[0].hypot(p[1])).fold(0.0, f64::max);
        assert!((far - 22.0).abs() < 1e-9);
        assert_eq!(g.preset(), 1, "NEMA 17");
    }

    #[test]
    fn helical_and_herringbone_gears_are_lofts_of_the_turned_outline() {
        let mut g = Gear::new(GearKind::Helical);
        assert_eq!(g.helix, 20.0);
        let SolidOp::Loft {
            sections, ruled, ..
        } = &g.ops()[0]
        else {
            panic!("a loft");
        };
        assert!(*ruled);
        assert!(sections.len() >= 2);
        g.kind = GearKind::Herringbone;
        let SolidOp::Loft { sections, .. } = &g.ops()[0] else {
            panic!("a loft");
        };
        assert_eq!(sections.len(), 3);
        assert!(g.mesh_text().contains("left-handed too"));
    }

    #[test]
    fn every_kind_builds_with_its_hub_and_set_screw() {
        for kind in GearKind::ALL {
            let mut g = Gear::new(kind);
            assert_eq!(g.problem(), None, "{}", kind.name());
            assert!(!g.ops().is_empty());
            if kind.has_bore() {
                g.hub = 12.0;
                g.hub_h = 8.0;
                g.set_screw = 3.0;
                assert_eq!(g.problem(), None, "{} with a hub", kind.name());
                let cuts = g
                    .ops()
                    .iter()
                    .filter(|op| op.boolean_op() == Some(BooleanOp::Cut))
                    .count();
                assert!(cuts >= 2, "{}: the bore and the set screw", kind.name());
            }
            let ctx = Ctx {
                feature: "f",
                focus: Some("module"),
            };
            assert!(matches!(
                g.panel(&ctx).first(),
                Some(Widget::Diagram { .. })
            ));
        }
    }

    #[test]
    fn a_worm_cuts_one_groove_per_start_and_names_its_wheel() {
        let mut g = Gear::new(GearKind::Worm);
        g.starts = 2;
        g.bore = 0.0;
        assert_eq!(g.ops().len(), 3, "the cylinder and two grooves");
        assert!((g.lead_angle() - 18.43).abs() < 0.1, "{}", g.lead_angle());
        assert_eq!(g.centre_distance(), Some(26.0));
    }

    #[test]
    fn a_pulley_has_as_many_grooves_as_teeth_and_flanges() {
        let g = Gear::new(GearKind::Pulley);
        assert_eq!((g.thickness, g.hub, g.set_screw), (7.0, 12.0, 3.0));
        assert!((g.pitch_radius() * 2.0 - 12.73).abs() < 0.01);
        assert!((g.outer_radius() * 2.0 - 12.22).abs() < 0.01);
        assert_eq!(g.ops().len(), 6, "body, two flanges, hub, bore, set screw");
        assert_eq!(g.label(), "GT2 20T pulley");
    }

    #[test]
    fn a_command_names_its_gear() {
        let d = Defaults::default();
        let g = Gear::with_args(
            &json!({"kind": "rack", "module": 1.5, "length": 80, "thickness": 6}),
            &d,
        )
        .unwrap();
        assert_eq!((g.kind, g.length, g.thickness), (GearKind::Rack, 80.0, 6.0));
        assert_eq!(g.label(), "m1.5 rack × 80");
        let g = Gear::with_args(
            &json!({"kind": "bevel", "teeth": 16, "mate": 32, "bore": 6, "shaft": "hex"}),
            &d,
        )
        .unwrap();
        assert_eq!(g.shaft, Shaft::Hex);
        assert_eq!(g.problem(), None);
        assert!(Gear::with_args(&json!({"kind": "sprocket"}), &d).is_err());
        assert!(Gear::with_args(&json!({"shaft": "oval"}), &d).is_err());
    }
}
