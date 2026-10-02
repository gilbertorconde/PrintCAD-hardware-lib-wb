//! Nuts: hex, thin, nylon-collared and square. A nut stands on `z = 0`
//! with its axis up Z. T-slot nuts are a family of their own (`tnut`).

use printcad_bench_sdk::Value;
use printcad_bench_sdk::api::kernel_api::{BooleanOp, SolidOp};
use printcad_bench_sdk::api::{DiagramStroke, PanelEvent, Parameter, Widget};
use serde::{Deserialize, Serialize};

use super::{
    Ctx, Defaults, Family, Part, arg_bool, arg_f64, arg_str, choice, fmt, group, index_of, length,
    note, number, toggle,
};
use crate::diagram::Sketch;
use std::f64::consts::PI;

use printcad_bench_sdk::api::kernel_api::ProfilePlane;

use crate::geom::{self, across_corners, revolve};
use crate::standards::{self, NutTable, internal_minor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NutKind {
    Hex,
    Thin,
    Nyloc,
    Square,
    /// Two wings on a round body, turned by hand.
    Wing,
    /// A knurled disc on a shoulder, turned by hand.
    Thumb,
}

impl NutKind {
    pub const ALL: [NutKind; 6] = [
        NutKind::Hex,
        NutKind::Thin,
        NutKind::Nyloc,
        NutKind::Square,
        NutKind::Wing,
        NutKind::Thumb,
    ];

    pub fn name(self) -> &'static str {
        match self {
            NutKind::Hex => "Hex",
            NutKind::Thin => "Thin hex",
            NutKind::Nyloc => "Nylon insert lock",
            NutKind::Square => "Square",
            NutKind::Wing => "Wing",
            NutKind::Thumb => "Knurled thumb",
        }
    }

    fn short(self) -> &'static str {
        match self {
            NutKind::Hex => "hex nut",
            NutKind::Thin => "thin nut",
            NutKind::Nyloc => "lock nut",
            NutKind::Square => "square nut",
            NutKind::Wing => "wing nut",
            NutKind::Thumb => "thumb nut",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            NutKind::Hex => "nut-hex",
            NutKind::Thin => "nut-thin",
            NutKind::Nyloc => "nut-nyloc",
            NutKind::Square => "nut-square",
            NutKind::Wing => "nut-wing",
            NutKind::Thumb => "nut-thumb",
        }
    }

    pub fn tool(self) -> &'static str {
        match self {
            NutKind::Hex => "hex_nut",
            NutKind::Thin => "thin_nut",
            NutKind::Nyloc => "nyloc_nut",
            NutKind::Square => "square_nut",
            NutKind::Wing => "wing_nut",
            NutKind::Thumb => "thumb_nut",
        }
    }

    pub fn named(name: &str) -> Option<NutKind> {
        NutKind::ALL.into_iter().find(|k| {
            k.tool().eq_ignore_ascii_case(name)
                || k.name().eq_ignore_ascii_case(name)
                || k.short().eq_ignore_ascii_case(name)
                || k.tool().trim_end_matches("_nut").eq_ignore_ascii_case(name)
        })
    }

    /// The standards that size this nut.
    pub fn standards(self) -> &'static [NutTable] {
        match self {
            NutKind::Hex => &[standards::ISO_4032],
            NutKind::Thin => &[standards::ISO_4035],
            NutKind::Nyloc => &[standards::DIN_985],
            NutKind::Square => &[standards::DIN_562, standards::DIN_557],
            NutKind::Wing => &[standards::DIN_315],
            NutKind::Thumb => &[standards::DIN_466],
        }
    }

    /// The row a size off the tables gets, in proportion to its thread.
    fn proportions(self, d: f64) -> (f64, f64, f64, f64, f64) {
        let r = |v: f64| (v * 10.0).round() / 10.0;
        match self {
            NutKind::Hex => (r(1.6 * d + 0.5), r(0.85 * d), 0.0, 0.0, 0.0),
            NutKind::Thin => (r(1.6 * d + 0.5), r(0.5 * d), 0.0, 0.0, 0.0),
            NutKind::Nyloc => (r(1.6 * d + 0.5), r(1.1 * d), 0.0, 0.0, 0.0),
            NutKind::Square => (r(1.6 * d + 0.5), r(0.8 * d), 0.0, 0.0, 0.0),
            NutKind::Wing => (r(1.8 * d), r(2.5 * d), r(5.0 * d), r(0.45 * d), r(d)),
            NutKind::Thumb => (r(2.0 * d), r(2.4 * d), r(4.0 * d), 0.0, r(0.8 * d)),
        }
    }
}

/// A nut's data. Lengths in millimetres.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Nut {
    pub kind: NutKind,
    pub standard: String,
    pub size: String,
    pub d: f64,
    pub pitch: f64,
    /// Width across flats; a square nut's side.
    pub s: f64,
    /// Height over all.
    pub m: f64,
    /// A lock nut's collar: its diameter and height.
    #[serde(default)]
    pub collar_d: f64,
    #[serde(default)]
    pub collar_h: f64,
    /// A wing nut's span and wing thickness, or a thumb nut's knurled disc
    /// across; and the body's height under the wings, or the disc's.
    #[serde(default)]
    pub e: f64,
    #[serde(default)]
    pub g: f64,
    #[serde(default)]
    pub k: f64,
    #[serde(default)]
    pub custom: bool,
    #[serde(default)]
    pub thread: bool,
}

impl Nut {
    pub fn new(kind: NutKind, size: &str, defaults: &Defaults) -> Nut {
        let mut nut = Nut {
            kind,
            standard: kind.standards()[0].name.into(),
            size: size.into(),
            d: 0.0,
            pitch: 0.0,
            s: 0.0,
            m: 0.0,
            collar_d: 0.0,
            collar_h: 0.0,
            e: 0.0,
            g: 0.0,
            k: 0.0,
            custom: false,
            thread: defaults.thread,
        };
        nut.refill();
        nut
    }

    /// Size the nut `size`, a diameter off the tables, in proportion to
    /// its thread.
    fn proportion(&mut self, size: &str, d: f64, pitch: f64) {
        self.size = size.trim().to_uppercase();
        self.d = d;
        self.pitch = pitch;
        (self.s, self.m, self.e, self.g, self.k) = self.kind.proportions(d);
        if self.kind == NutKind::Nyloc {
            self.collar_h = ((0.4 * self.m) * 10.0).round() / 10.0;
            self.collar_d = ((0.92 * self.s) * 10.0).round() / 10.0;
        }
    }

    /// How many facets a knurled disc shows round.
    fn knurl_facets(&self) -> u32 {
        ((std::f64::consts::PI * self.e / 1.2).round() as u32).clamp(24, 60)
    }

    fn table(&self) -> Option<&'static NutTable> {
        let tables = self.kind.standards();
        tables
            .iter()
            .find(|t| t.name == self.standard)
            .or(tables.first())
    }

    fn standards(&self) -> Vec<String> {
        self.kind
            .standards()
            .iter()
            .map(|t| t.name.to_string())
            .collect()
    }

    fn sizes(&self) -> Vec<&'static str> {
        self.table()
            .map(|t| t.rows.iter().map(|r| r.size).collect())
            .unwrap_or_default()
    }

    /// Take the dimensions of the size from the standard, the nearest
    /// size it has when it lacks the one named.
    pub fn refill(&mut self) {
        let Some(table) = self.table() else {
            return;
        };
        self.standard = table.name.into();
        let row = table
            .rows
            .iter()
            .find(|r| r.size.eq_ignore_ascii_case(&self.size))
            .or_else(|| {
                let want = standards::metric(&self.size)
                    .map(|(d, _)| d)
                    .unwrap_or(self.d);
                table
                    .rows
                    .iter()
                    .min_by(|a, b| (a.d - want).abs().total_cmp(&(b.d - want).abs()))
            })
            .copied()
            .unwrap_or(table.rows[0]);
        self.size = row.size.into();
        self.d = row.d;
        self.pitch = row.pitch;
        self.s = row.s;
        self.m = row.m;
        self.e = row.e;
        self.g = row.g;
        self.k = row.k;
        if self.kind == NutKind::Nyloc {
            // The metal is a regular nut's height; the collar is the rest.
            let metal = standards::ISO_4032
                .rows
                .iter()
                .find(|r| r.size == row.size)
                .map_or(0.6 * row.m, |r| r.m);
            self.collar_h = (row.m - metal).max(0.2 * row.m);
            self.collar_d = 0.92 * row.s;
        }
    }

    fn minor(&self) -> f64 {
        internal_minor(self.d, self.pitch)
    }

    /// The height of the hex (or square) body, under any collar.
    fn body_height(&self) -> f64 {
        match self.kind {
            NutKind::Nyloc => self.m - self.collar_h,
            NutKind::Wing => self.k,
            NutKind::Thumb => self.m - self.k,
            _ => self.m,
        }
    }

    /// A wing's outline in `(x, z)`, from the body out to the span,
    /// rounded at its tip, for the wing on +X.
    fn wing(&self) -> Vec<[f64; 2]> {
        let (x0, xo, h) = (self.s / 2.0 - 0.3, self.e / 2.0, self.m);
        let z0 = 0.12 * h;
        let r = (0.3 * (xo - x0)).min(0.3 * h).max(0.01);
        let mut pts = vec![[x0, z0], [xo - r, z0]];
        pts.extend(geom::arc_points([xo - r, z0 + r], r, -PI / 2.0, 0.0, 4));
        pts.push([xo, h - r]);
        pts.extend(geom::arc_points([xo - r, h - r], r, 0.0, PI / 2.0, 4));
        pts.push([x0 + 0.3 * (xo - x0), h]);
        pts.push([x0, 0.8 * h]);
        pts
    }
}

impl Part for Nut {
    const FAMILY: Family = Family::Nut;

    fn label(&self) -> String {
        format!("{} {}", self.size, self.kind.short())
    }

    fn icon(&self) -> &'static str {
        self.kind.icon()
    }

    fn problem(&self) -> Option<String> {
        if self.d <= 0.0 || self.pitch <= 0.0 || self.m <= 0.0 || self.s <= 0.0 {
            return Some("The thread, the width and the height must be more than 0.".into());
        }
        if self.minor() <= 0.0 {
            return Some("The pitch is too coarse for the thread's diameter.".into());
        }
        if self.s <= self.d {
            return Some("The nut must be wider than its thread.".into());
        }
        if self.kind == NutKind::Wing
            && (self.e <= self.s || self.g <= 0.0 || self.k >= self.m || self.k <= 0.0)
        {
            return Some("The wings must span past the body, with a thickness, over a body lower than the nut.".into());
        }
        if self.kind == NutKind::Thumb && (self.e <= self.s || self.k >= self.m || self.k <= 0.0) {
            return Some(
                "The knurl must be wider than the shoulder and lower than the nut.".into(),
            );
        }
        if self.kind == NutKind::Nyloc
            && (self.collar_h <= 0.0
                || self.collar_h >= self.m
                || self.collar_d <= self.d
                || self.collar_d > self.s)
        {
            return Some(
                "The collar must be narrower than the nut and shorter than its height.".into(),
            );
        }
        None
    }

    fn axis(&self) -> ([f64; 3], [f64; 3]) {
        super::z_axis(0.0, self.m)
    }

    fn ops(&self) -> Vec<SolidOp> {
        let bore = vec![geom::circle(self.minor())];
        let h = self.body_height();
        let mut ops = match self.kind {
            NutKind::Hex | NutKind::Thin | NutKind::Nyloc => {
                let e = across_corners(self.s);
                let c = (0.1 * self.s).min(0.25 * h);
                let mut ops = vec![
                    geom::prism(&geom::hexagon(self.s), bore, 0.0, h, BooleanOp::NewSolid),
                    geom::crown(e, 0.0, h, c, 30.0, true, self.kind != NutKind::Nyloc),
                ];
                if self.kind == NutKind::Nyloc {
                    let (r0, r1, top) = (self.minor() / 2.0, self.collar_d / 2.0, self.m);
                    let cc = (0.15 * self.collar_h).min(0.4);
                    ops.push(revolve(
                        &[
                            [r0, h - 0.2],
                            [r1, h - 0.2],
                            [r1, top - cc],
                            [r1 - cc, top],
                            [r0, top],
                        ],
                        BooleanOp::Fuse,
                    ));
                }
                ops
            }
            NutKind::Square => vec![geom::prism(
                &geom::square(self.s),
                bore,
                0.0,
                h,
                BooleanOp::NewSolid,
            )],
            NutKind::Wing => {
                // A round body, a wing fused on each side of it, each a
                // plate drawn in the XZ plane and extruded through Y.
                let mut ops = vec![geom::prism(
                    &geom::regular(48, self.s),
                    bore,
                    0.0,
                    h,
                    BooleanOp::NewSolid,
                )];
                let wing = self.wing();
                for sign in [1.0, -1.0] {
                    let outline: Vec<[f64; 2]> = if sign > 0.0 {
                        wing.clone()
                    } else {
                        wing.iter().rev().map(|p| [-p[0], p[1]]).collect()
                    };
                    let plane = ProfilePlane {
                        origin: [0.0, self.g / 2.0, 0.0],
                        x_axis: [1.0, 0.0, 0.0],
                        y_axis: [0.0, 0.0, 1.0],
                        normal: [0.0, -1.0, 0.0],
                    };
                    ops.push(geom::extrude(
                        plane,
                        vec![geom::polygon(&outline)],
                        self.g,
                        BooleanOp::Fuse,
                    ));
                }
                ops
            }
            NutKind::Thumb => {
                // The shoulder, with the knurled disc fused on its top and
                // a little down into it.
                let n = self.knurl_facets();
                let across = self.e * (PI / f64::from(n)).cos();
                let overlap = (self.k / 2.0).min(0.5);
                vec![
                    geom::prism(
                        &geom::regular(48, self.s),
                        bore.clone(),
                        0.0,
                        h + overlap,
                        BooleanOp::NewSolid,
                    ),
                    geom::prism(&geom::regular(n, across), bore, h, self.k, BooleanOp::Fuse),
                ]
            }
        };
        if self.thread {
            let top = self.m;
            ops.push(geom::internal_thread(
                self.d,
                self.minor(),
                self.pitch,
                top,
                self.m,
            ));
        }
        ops
    }

    fn panel(&self, ctx: &Ctx) -> Vec<Widget> {
        let kinds: Vec<&str> = NutKind::ALL.iter().map(|k| k.name()).collect();
        let standards = self.standards();
        let sizes = self.sizes();
        let mut widgets = vec![
            self.drawing(ctx),
            choice(
                "kind",
                "Nut",
                &kinds,
                NutKind::ALL
                    .iter()
                    .position(|k| *k == self.kind)
                    .unwrap_or(0),
            ),
            choice(
                "standard",
                "Standard",
                &standards,
                index_of(&standards, &self.standard),
            ),
            choice("size", "Thread", &sizes, index_of(&sizes, &self.size)),
        ];
        let mut dims = vec![toggle("custom", "Custom dimensions", self.custom)];
        if self.custom {
            dims.push(number(ctx, "d", "Thread diameter", self.d, 0.1, 2));
            dims.push(number(ctx, "pitch", "Pitch", self.pitch, 0.05, 2));
            let width = match self.kind {
                NutKind::Square => "Side",
                NutKind::Wing => "Body diameter",
                NutKind::Thumb => "Shoulder diameter",
                _ => "Across flats",
            };
            dims.push(number(ctx, "s", width, self.s, 0.1, 2));
            dims.push(number(ctx, "m", "Height", self.m, 0.1, 2));
            if self.kind == NutKind::Wing {
                dims.push(number(ctx, "e", "Wing span", self.e, 0.1, 2));
                dims.push(number(ctx, "g", "Wing thickness", self.g, 0.1, 2));
                dims.push(number(ctx, "k", "Body height", self.k, 0.1, 2));
            }
            if self.kind == NutKind::Thumb {
                dims.push(number(ctx, "e", "Knurl diameter", self.e, 0.1, 2));
                dims.push(number(ctx, "k", "Knurl height", self.k, 0.1, 2));
            }
            if self.kind == NutKind::Nyloc {
                dims.push(number(
                    ctx,
                    "collar_d",
                    "Collar diameter",
                    self.collar_d,
                    0.1,
                    2,
                ));
                dims.push(number(
                    ctx,
                    "collar_h",
                    "Collar height",
                    self.collar_h,
                    0.1,
                    2,
                ));
            }
        }
        widgets.push(group("Dimensions", self.custom, dims));
        let mut options = vec![toggle("thread", "Modelled thread", self.thread)];
        if self.thread {
            options.push(super::text("A modelled thread takes the kernel a while."));
        }
        widgets.push(group("Options", true, options));
        widgets.extend(note(self.problem()));
        widgets
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            length("d", "Thread diameter"),
            length("pitch", "Pitch"),
            length("s", "Width"),
            length("m", "Height"),
            length("collar_d", "Collar diameter"),
            length("collar_h", "Collar height"),
            length("e", "Span"),
            length("g", "Wing thickness"),
            length("k", "Body height"),
        ]
    }

    fn apply(&mut self, event: &PanelEvent) -> bool {
        match event {
            PanelEvent::Choice { id, index } => match id.as_str() {
                "kind" => {
                    self.kind = NutKind::ALL.get(*index).copied().unwrap_or(NutKind::Hex);
                    self.standard = self.standards().first().cloned().unwrap_or_default();
                    self.refill();
                }
                "standard" => {
                    if let Some(standard) = self.standards().get(*index) {
                        self.standard = standard.clone();
                        self.refill();
                    }
                }
                "size" => {
                    if let Some(size) = self.sizes().get(*index) {
                        self.size = (*size).into();
                        self.refill();
                    }
                }
                _ => return false,
            },
            PanelEvent::Number { id, value } => match id.as_str() {
                "d" => self.d = *value,
                "pitch" => self.pitch = *value,
                "s" => self.s = *value,
                "m" => self.m = *value,
                "collar_d" => self.collar_d = *value,
                "collar_h" => self.collar_h = *value,
                "e" => self.e = *value,
                "g" => self.g = *value,
                "k" => self.k = *value,
                _ => return false,
            },
            PanelEvent::Toggle { id, on } => match id.as_str() {
                "custom" => {
                    self.custom = *on;
                    if !*on {
                        self.refill();
                    }
                }
                "thread" => self.thread = *on,
                _ => return false,
            },
            _ => return false,
        }
        true
    }

    fn with_args(args: &Value, defaults: &Defaults) -> Result<Self, String> {
        let kind = match arg_str(args, "kind") {
            None => NutKind::Hex,
            Some(name) => NutKind::named(name).ok_or_else(|| {
                format!(
                    "`{name}` is not a nut: {}",
                    NutKind::ALL.map(|k| k.tool()).join(", ")
                )
            })?,
        };
        let size = arg_str(args, "size").unwrap_or(&defaults.size);
        let mut nut = Nut::new(kind, size, defaults);
        if let Some(standard) = arg_str(args, "standard") {
            let found = nut
                .standards()
                .into_iter()
                .find(|s| s.eq_ignore_ascii_case(standard) || s.starts_with(standard))
                .ok_or_else(|| format!("`{standard}` does not size a {}", kind.short()))?;
            nut.standard = found;
            nut.refill();
        }
        nut.size = size.into();
        nut.refill();
        if !nut.size.eq_ignore_ascii_case(size) && arg_str(args, "size").is_some() {
            // A size the table lacks is made in proportion to its thread.
            let (d, pitch) = standards::metric_any(size)
                .ok_or_else(|| format!("{} has no size `{size}`", nut.standard))?;
            nut.proportion(size, d, pitch);
        }
        if arg_str(args, "size").is_none()
            && let Some(d) = arg_f64(args, "d")
            && (d - nut.d).abs() > 1e-9
        {
            let name = standards::size_name(d);
            if nut.sizes().iter().any(|s| s.eq_ignore_ascii_case(&name)) {
                nut.size = name;
                nut.refill();
            } else {
                nut.proportion(&name, d, standards::coarse_pitch(d));
            }
        }
        for (key, slot) in [
            ("d", &mut nut.d),
            ("pitch", &mut nut.pitch),
            ("s", &mut nut.s),
            ("m", &mut nut.m),
            ("collar_d", &mut nut.collar_d),
            ("collar_h", &mut nut.collar_h),
            ("e", &mut nut.e),
            ("g", &mut nut.g),
            ("k", &mut nut.k),
        ] {
            if let Some(v) = arg_f64(args, key) {
                *slot = v;
                nut.custom = true;
            }
        }
        if let Some(on) = arg_bool(args, "thread") {
            nut.thread = on;
        }
        Ok(nut)
    }
}

impl Nut {
    /// The side view, the bore behind the surface.
    fn drawing(&self, ctx: &Ctx) -> Widget {
        if matches!(self.kind, NutKind::Wing | NutKind::Thumb) {
            return self.hand_drawing(ctx);
        }
        let mut s = Sketch::new(ctx.focus);
        let m = self.m;
        let r = self.d / 2.0;
        let wide = match self.kind {
            NutKind::Square => self.s,
            _ => across_corners(self.s),
        };
        let off = Sketch::standoff(wide.max(m));
        let w = wide / 2.0;
        let h = self.body_height();
        s.rect([-w, 0.0], [w, h]);
        if self.kind != NutKind::Square {
            let a = w / 2.0;
            s.line(&[[-a, 0.0], [-a, h]], DiagramStroke::Outline);
            s.line(&[[a, 0.0], [a, h]], DiagramStroke::Outline);
        }
        if self.kind == NutKind::Nyloc {
            let c = self.collar_d / 2.0;
            s.rect([-c, h], [c, m]);
            s.height("collar_h", w, h, m, -off, fmt(self.collar_h));
            s.width("collar_d", -c, c, m, off, fmt(self.collar_d));
        } else {
            let label = if self.kind == NutKind::Square {
                "a"
            } else {
                "s"
            };
            s.width("s", -w, w, m, off, format!("{label} {}", fmt(self.s)));
        }
        s.height("m", -w, 0.0, m, off, format!("m {}", fmt(m)));
        s.hidden(&[[-r, 0.0], [-r, m]]);
        s.hidden(&[[r, 0.0], [r, m]]);
        s.axis(0.0, -off * 0.4, m + off * 0.4);
        s.width(
            "d",
            -r,
            r,
            0.0,
            -off,
            format!("{} × {}", self.size, fmt(self.pitch)),
        );
        s.finish("nut")
    }

    /// A wing or thumb nut's side view: the body and what is turned by
    /// hand on it.
    fn hand_drawing(&self, ctx: &Ctx) -> Widget {
        let mut s = Sketch::new(ctx.focus);
        let (m, r, half) = (self.m, self.d / 2.0, self.e / 2.0);
        let off = Sketch::standoff(self.e.max(m));
        let body = self.s / 2.0;
        match self.kind {
            NutKind::Wing => {
                s.rect([-body, 0.0], [body, self.k]);
                let wing = self.wing();
                let other: Vec<[f64; 2]> = wing.iter().map(|p| [-p[0], p[1]]).collect();
                s.poly(&wing, DiagramStroke::Outline, false);
                s.poly(&other, DiagramStroke::Outline, false);
                s.width("e", -half, half, m, off, format!("e {}", fmt(self.e)));
                s.width("s", -body, body, 0.0, -off, format!("d2 {}", fmt(self.s)));
                s.height(
                    "k",
                    -half,
                    0.0,
                    self.k,
                    off * 0.6,
                    format!("k {}", fmt(self.k)),
                );
                s.callout(
                    "g",
                    [half - 0.5, m / 2.0],
                    [half + off, m / 2.0 + off],
                    format!("g {}", fmt(self.g)),
                );
            }
            _ => {
                let top = m - self.k;
                s.rect([-body, 0.0], [body, top]);
                s.rect([-half, top], [half, m]);
                let n = 11;
                for i in 1..n {
                    let x = -half + self.e * i as f64 / n as f64;
                    s.line(&[[x, top], [x, m]], DiagramStroke::Thin);
                }
                s.width("e", -half, half, m, off, format!("dk {}", fmt(self.e)));
                s.width("s", -body, body, 0.0, -off, format!("ds {}", fmt(self.s)));
                s.height("k", half, top, m, -off * 0.6, format!("k {}", fmt(self.k)));
            }
        }
        s.height("m", -half, 0.0, m, off * 1.4, format!("h {}", fmt(m)));
        s.hidden(&[[-r, 0.0], [-r, m]]);
        s.hidden(&[[r, 0.0], [r, m]]);
        s.axis(0.0, -off * 0.4, m + off * 0.4);
        s.callout(
            "d",
            [r, m * 0.3],
            [half + off * 0.8, -off * 0.6],
            format!("{} × {}", self.size, fmt(self.pitch)),
        );
        s.finish("nut")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printcad_bench_sdk::json;

    fn m3(kind: NutKind) -> Nut {
        Nut::new(kind, "M3", &Defaults::default())
    }

    #[test]
    fn a_hex_nut_is_sized_by_its_standard_and_crowned() {
        let nut = m3(NutKind::Hex);
        assert_eq!((nut.s, nut.m), (5.5, 2.4));
        assert_eq!(nut.label(), "M3 hex nut");
        let roles: Vec<_> = nut.ops().iter().map(|op| op.boolean_op()).collect();
        assert_eq!(roles, [Some(BooleanOp::NewSolid), Some(BooleanOp::Common)]);
    }

    #[test]
    fn every_nut_builds() {
        for kind in NutKind::ALL {
            let nut = m3(kind);
            assert_eq!(nut.problem(), None, "{}", kind.name());
            assert!(!nut.ops().is_empty());
            let ctx = Ctx {
                feature: "f",
                focus: Some("m"),
            };
            let panel = nut.panel(&ctx);
            assert!(matches!(panel.first(), Some(Widget::Diagram { .. })));
        }
    }

    #[test]
    fn a_lock_nut_has_a_collar_over_a_regular_nuts_height() {
        let nut = m3(NutKind::Nyloc);
        assert!(
            (nut.collar_h - 1.6).abs() < 1e-9,
            "4 over all, 2.4 of metal: {}",
            nut.collar_h
        );
        assert_eq!(nut.ops().len(), 3);
    }

    #[test]
    fn a_square_nut_offers_two_standards() {
        let mut nut = Nut::new(NutKind::Square, "M6", &Defaults::default());
        assert_eq!(nut.m, 3.2);
        assert!(nut.apply(&PanelEvent::Choice {
            id: "standard".into(),
            index: 1,
        }));
        assert_eq!((nut.standard.as_str(), nut.m), ("DIN 557", 5.0));
    }

    #[test]
    fn a_command_names_its_nut() {
        let nut = Nut::with_args(
            &json!({"kind": "nyloc", "size": "M5"}),
            &Defaults::default(),
        )
        .unwrap();
        assert_eq!((nut.kind, nut.s, nut.m), (NutKind::Nyloc, 8.0, 5.0));
        assert!(Nut::with_args(&json!({"kind": "flange"}), &Defaults::default()).is_err());
    }

    #[test]
    fn a_wing_nut_and_a_thumb_nut_are_their_standards() {
        let wing = Nut::new(NutKind::Wing, "M6", &Defaults::default());
        assert_eq!(
            (wing.s, wing.m, wing.e, wing.g, wing.k),
            (11.5, 16.0, 31.5, 2.5, 6.5)
        );
        assert_eq!(wing.label(), "M6 wing nut");
        assert_eq!(wing.problem(), None);
        assert_eq!(wing.ops().len(), 3, "the body and two wings");
        let thumb = Nut::new(NutKind::Thumb, "M3", &Defaults::default());
        assert_eq!((thumb.s, thumb.m, thumb.e, thumb.k), (6.0, 7.5, 12.0, 2.5));
        assert_eq!(thumb.ops().len(), 2);
        // M3 is under the wing nuts' table: the nearest size, M4, is taken.
        let small = Nut::new(NutKind::Wing, "M3", &Defaults::default());
        assert_eq!(small.size, "M4");
    }

    #[test]
    fn a_size_off_the_table_is_made_in_proportion() {
        let nut = Nut::with_args(&json!({"size": "M7"}), &Defaults::default()).unwrap();
        assert_eq!(
            (nut.size.as_str(), nut.d, nut.pitch, nut.s, nut.m),
            ("M7", 7.0, 1.0, 11.7, 6.0)
        );
        assert_eq!(nut.problem(), None);
        let wing =
            Nut::with_args(&json!({"kind": "wing", "d": 3.0}), &Defaults::default()).unwrap();
        assert_eq!((wing.size.as_str(), wing.e, wing.m), ("M3", 15.0, 7.5));
        assert_eq!(wing.problem(), None);
    }
}
