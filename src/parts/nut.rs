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
use crate::geom::{self, across_corners, revolve};
use crate::standards::{self, NutTable, internal_minor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NutKind {
    Hex,
    Thin,
    Nyloc,
    Square,
}

impl NutKind {
    pub const ALL: [NutKind; 4] = [NutKind::Hex, NutKind::Thin, NutKind::Nyloc, NutKind::Square];

    pub fn name(self) -> &'static str {
        match self {
            NutKind::Hex => "Hex",
            NutKind::Thin => "Thin hex",
            NutKind::Nyloc => "Nylon insert lock",
            NutKind::Square => "Square",
        }
    }

    fn short(self) -> &'static str {
        match self {
            NutKind::Hex => "hex nut",
            NutKind::Thin => "thin nut",
            NutKind::Nyloc => "lock nut",
            NutKind::Square => "square nut",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            NutKind::Hex => "nut-hex",
            NutKind::Thin => "nut-thin",
            NutKind::Nyloc => "nut-nyloc",
            NutKind::Square => "nut-square",
        }
    }

    pub fn tool(self) -> &'static str {
        match self {
            NutKind::Hex => "hex_nut",
            NutKind::Thin => "thin_nut",
            NutKind::Nyloc => "nyloc_nut",
            NutKind::Square => "square_nut",
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
            custom: false,
            thread: defaults.thread,
        };
        nut.refill();
        nut
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
            _ => self.m,
        }
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
                _ => "Across flats",
            };
            dims.push(number(ctx, "s", width, self.s, 0.1, 2));
            dims.push(number(ctx, "m", "Height", self.m, 0.1, 2));
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
            return Err(format!("{} has no size `{size}`", nut.standard));
        }
        for (key, slot) in [
            ("d", &mut nut.d),
            ("pitch", &mut nut.pitch),
            ("s", &mut nut.s),
            ("m", &mut nut.m),
            ("collar_d", &mut nut.collar_d),
            ("collar_h", &mut nut.collar_h),
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
        assert!(Nut::with_args(&json!({"kind": "wing"}), &Defaults::default()).is_err());
    }
}
