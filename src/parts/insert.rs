//! Heat-set threaded inserts, as pressed into printed parts: a knurled
//! brass sleeve with a thread through it, on `z = 0` with its axis up Z,
//! the lead-in taper at the bottom.

use printcad_bench_sdk::Value;
use printcad_bench_sdk::api::kernel_api::{BooleanOp, SolidOp};
use printcad_bench_sdk::api::{DiagramStroke, PanelEvent, Parameter, Widget};
use serde::{Deserialize, Serialize};

use super::{
    Ctx, Defaults, Family, Part, arg_bool, arg_f64, arg_str, choice, fmt, group, index_of, length,
    note, number, text, toggle,
};
use crate::diagram::Sketch;
use crate::geom::{self, revolve};
use crate::standards::{self, InsertRow, internal_minor};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Insert {
    pub size: String,
    pub d: f64,
    pub pitch: f64,
    /// Across the upper knurl.
    pub outer: f64,
    /// Across the pilot end; 0 in a part saved before it was drawn.
    #[serde(default)]
    pub pilot: f64,
    pub length: f64,
    /// The hole it is driven into.
    pub hole: f64,
    /// The short length of the size rather than the standard one.
    #[serde(default)]
    pub short: bool,
    #[serde(default)]
    pub custom: bool,
    #[serde(default)]
    pub thread: bool,
}

impl Insert {
    pub fn new(size: &str, defaults: &Defaults) -> Insert {
        let mut insert = Insert {
            size: size.into(),
            d: 0.0,
            pitch: 0.0,
            outer: 0.0,
            pilot: 0.0,
            length: 0.0,
            hole: 0.0,
            short: false,
            custom: false,
            thread: defaults.thread,
        };
        insert.refill();
        insert
    }

    fn row(&self) -> InsertRow {
        standards::INSERTS
            .iter()
            .find(|r| r.size.eq_ignore_ascii_case(&self.size))
            .or_else(|| {
                let want = standards::metric(&self.size)
                    .map(|(d, _)| d)
                    .unwrap_or(self.d);
                standards::INSERTS
                    .iter()
                    .min_by(|a, b| (a.d - want).abs().total_cmp(&(b.d - want).abs()))
            })
            .copied()
            .unwrap_or(standards::INSERTS[2])
    }

    pub fn refill(&mut self) {
        let row = self.row();
        self.size = row.size.into();
        self.d = row.d;
        self.pitch = row.pitch;
        self.outer = row.outer;
        self.pilot = row.pilot;
        self.length = if self.short { row.short } else { row.length };
        self.hole = row.hole;
    }

    /// The pilot's diameter, a little under the knurl's when none was
    /// saved.
    fn pilot(&self) -> f64 {
        if self.pilot > 0.0 {
            self.pilot
        } else {
            self.outer - 0.4
        }
    }

    /// Where the bands lie up the insert, as shares of its length: the
    /// pilot, the lower knurl, the groove, the upper knurl, the collar.
    const PILOT: f64 = 0.12;
    const LOWER: f64 = 0.45;
    const GROOVE: f64 = 0.55;
    const UPPER: f64 = 0.92;

    /// How many facets a knurl band has round the insert.
    fn facets(&self) -> u32 {
        ((std::f64::consts::PI * self.outer / 0.9).round() as u32).clamp(12, 24)
    }

    fn minor(&self) -> f64 {
        internal_minor(self.d, self.pitch)
    }

    fn sizes() -> Vec<&'static str> {
        standards::INSERTS.iter().map(|r| r.size).collect()
    }
}

impl Part for Insert {
    const FAMILY: Family = Family::Insert;

    fn label(&self) -> String {
        format!("{} insert", self.size)
    }

    fn icon(&self) -> &'static str {
        "insert"
    }

    fn problem(&self) -> Option<String> {
        if self.d <= 0.0 || self.pitch <= 0.0 || self.length <= 0.0 {
            return Some("The thread and the length must be more than 0.".into());
        }
        if self.pilot() <= self.d + 0.4 {
            return Some("The wall around the thread is thinner than 0.2 mm.".into());
        }
        if self.outer <= self.pilot() {
            return Some("The knurl must be wider than the pilot.".into());
        }
        if self.minor() <= 0.0 {
            return Some("The pitch is too coarse for the thread's diameter.".into());
        }
        None
    }

    fn axis(&self) -> ([f64; 3], [f64; 3]) {
        super::z_axis(0.0, self.length)
    }

    fn ops(&self) -> Vec<SolidOp> {
        let (r0, rp, ro, l) = (
            self.minor() / 2.0,
            self.pilot() / 2.0,
            self.outer / 2.0,
            self.length,
        );
        let (z1, z2, z3, z4) = (
            Self::PILOT * l,
            Self::LOWER * l,
            Self::GROOVE * l,
            Self::UPPER * l,
        );
        // The lower knurl rises from the pilot's radius; the upper from a
        // root below the knurl's crest. The groove and the collar are
        // plain.
        let rib = ((ro - rp) * 0.6).clamp(0.1, 0.25);
        let lower_crest = rp + rib;
        let (upper_root, upper_crest) = (ro - rib, ro);
        let lead = (0.3f64).min(l / 8.0).min((rp - r0) * 0.8);
        let groove = (0.1f64).min((rp - r0) * 0.3);
        let mut ops = vec![revolve(
            &[
                [r0, 0.0],
                [rp - lead, 0.0],
                [rp, lead],
                [rp, z2],
                [rp - groove, z2],
                [rp - groove, z3],
                [upper_root, z3],
                [upper_root, l],
                [r0, l],
            ],
            BooleanOp::NewSolid,
        )];
        // The thread first, into the plain core: cut after the bands are on,
        // it is one the kernel's boolean will not resolve.
        if self.thread {
            ops.push(geom::internal_thread(
                self.d,
                self.minor(),
                self.pitch,
                l,
                l,
            ));
        }
        // The knurls: a faceted band fused on each, its corners at the
        // crest and its flats a little under, with its hole within the
        // wall, clear of the bore's face. (A serrated ring, true ribs, is
        // what the kernel's boolean will not fuse on.) Each band stops a
        // hair short of the step the core makes at the groove, so no face
        // of one lies in a face of the other.
        let hole = r0 + (rp - r0) / 2.0;
        let gap = 0.05;
        for (crest, from, to) in [(lower_crest, z1, z2 - gap), (upper_crest, z3 + gap, z4)] {
            let across = 2.0 * crest * (std::f64::consts::PI / f64::from(self.facets())).cos();
            ops.push(geom::prism(
                &geom::regular(self.facets(), across),
                vec![geom::circle(2.0 * hole)],
                from,
                to - from,
                BooleanOp::Fuse,
            ));
        }
        ops
    }

    fn panel(&self, ctx: &Ctx) -> Vec<Widget> {
        let sizes = Self::sizes();
        let row = self.row();
        let mut widgets = vec![
            self.drawing(ctx),
            choice("size", "Thread", &sizes, index_of(&sizes, &self.size)),
        ];
        // A size made short as well offers the choice.
        if row.short < row.length {
            widgets.push(choice(
                "short",
                "Length",
                &[
                    format!("Standard ({})", fmt(row.length)),
                    format!("Short ({})", fmt(row.short)),
                ],
                usize::from(self.short),
            ));
        }
        widgets.push(text(format!(
            "Drive it into a Ø{} hole, {} deep or more.",
            fmt(self.hole),
            fmt(self.length + 1.0)
        )));
        let mut dims = vec![toggle("custom", "Custom dimensions", self.custom)];
        if self.custom {
            dims.push(number(ctx, "d", "Thread diameter", self.d, 0.1, 2));
            dims.push(number(ctx, "pitch", "Pitch", self.pitch, 0.05, 2));
            dims.push(number(ctx, "outer", "Knurl diameter", self.outer, 0.1, 2));
            dims.push(number(ctx, "pilot", "Pilot diameter", self.pilot(), 0.1, 2));
            dims.push(number(ctx, "length", "Length", self.length, 0.1, 2));
            dims.push(number(ctx, "hole", "Hole", self.hole, 0.1, 2));
        }
        widgets.push(group("Dimensions", self.custom, dims));
        widgets.push(group(
            "Options",
            true,
            vec![toggle("thread", "Modelled thread", self.thread)],
        ));
        widgets.extend(note(self.problem()));
        widgets
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            length("d", "Thread diameter"),
            length("pitch", "Pitch"),
            length("outer", "Knurl diameter"),
            length("pilot", "Pilot diameter"),
            length("length", "Length"),
            length("hole", "Hole"),
        ]
    }

    fn apply(&mut self, event: &PanelEvent) -> bool {
        match event {
            PanelEvent::Choice { id, index } => match id.as_str() {
                "size" => {
                    if let Some(size) = Self::sizes().get(*index) {
                        self.size = (*size).into();
                        self.refill();
                    }
                }
                "short" => {
                    self.short = *index == 1;
                    if !self.custom {
                        self.refill();
                    } else {
                        let row = self.row();
                        self.length = if self.short { row.short } else { row.length };
                    }
                }
                _ => return false,
            },
            PanelEvent::Number { id, value } => match id.as_str() {
                "d" => self.d = *value,
                "pitch" => self.pitch = *value,
                "outer" => self.outer = *value,
                "pilot" => self.pilot = *value,
                "length" => self.length = *value,
                "hole" => self.hole = *value,
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
        let size = arg_str(args, "size").unwrap_or(&defaults.size);
        let mut insert = Insert::new(size, defaults);
        if !insert.size.eq_ignore_ascii_case(size) && arg_str(args, "size").is_some() {
            return Err(format!(
                "no insert is made in `{size}`; sizes: {}",
                Self::sizes().join(", ")
            ));
        }
        if let Some(short) = arg_bool(args, "short") {
            insert.short = short;
            insert.refill();
        }
        for (key, slot) in [
            ("d", &mut insert.d),
            ("pitch", &mut insert.pitch),
            ("outer", &mut insert.outer),
            ("pilot", &mut insert.pilot),
            ("length", &mut insert.length),
            ("hole", &mut insert.hole),
        ] {
            if let Some(v) = arg_f64(args, key) {
                *slot = v;
                insert.custom = true;
            }
        }
        if let Some(on) = arg_bool(args, "thread") {
            insert.thread = on;
        }
        Ok(insert)
    }
}

impl Insert {
    fn drawing(&self, ctx: &Ctx) -> Widget {
        let mut s = Sketch::new(ctx.focus);
        let (r0, rp, ro, l) = (
            self.d / 2.0,
            self.pilot() / 2.0,
            self.outer / 2.0,
            self.length,
        );
        let (z1, z2, z3, z4) = (
            Self::PILOT * l,
            Self::LOWER * l,
            Self::GROOVE * l,
            Self::UPPER * l,
        );
        let off = Sketch::standoff(self.outer.max(l));
        let rib = ((ro - rp) * 0.6).clamp(0.1, 0.25);
        // The silhouette: pilot, lower knurl, groove, upper knurl, collar.
        s.poly(
            &[
                [-rp, 0.0],
                [rp, 0.0],
                [rp, z1],
                [rp + rib, z1],
                [rp + rib, z2],
                [rp, z2],
                [rp, z3],
                [ro, z3],
                [ro, z4],
                [ro - rib, z4],
                [ro - rib, l],
                [-(ro - rib), l],
                [-(ro - rib), z4],
                [-ro, z4],
                [-ro, z3],
                [-rp, z3],
                [-rp, z2],
                [-(rp + rib), z2],
                [-(rp + rib), z1],
                [-rp, z1],
            ],
            DiagramStroke::Outline,
            false,
        );
        // The facets, a few of each knurl.
        for (half, from, to) in [(rp + rib, z1, z2), (ro, z3, z4)] {
            for i in 1..5 {
                let x = -half + 2.0 * half * i as f64 / 5.0;
                s.line(&[[x, from], [x, to]], DiagramStroke::Thin);
            }
        }
        s.hidden(&[[-r0, 0.0], [-r0, l]]);
        s.hidden(&[[r0, 0.0], [r0, l]]);
        s.axis(0.0, -off * 0.4, l + off * 0.4);
        s.width("outer", -ro, ro, l, off, format!("Ø{}", fmt(self.outer)));
        s.width(
            "pilot",
            -rp,
            rp,
            0.0,
            -off,
            format!("Ø{}", fmt(self.pilot())),
        );
        s.height("length", ro, 0.0, l, -off, format!("L {}", fmt(l)));
        s.callout(
            "d",
            [r0, l * 0.75],
            [-ro - off * 1.8, l + off * 0.6],
            format!("{} × {}", self.size, fmt(self.pitch)),
        );
        s.callout(
            "hole",
            [-rp, z1 * 0.5],
            [-ro - off * 1.8, -off * 0.6],
            format!("hole Ø{}", fmt(self.hole)),
        );
        s.finish("insert")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printcad_bench_sdk::json;

    #[test]
    fn an_m3_insert_is_sized_and_names_its_hole() {
        let insert = Insert::new("M3", &Defaults::default());
        assert_eq!(
            (insert.outer, insert.pilot, insert.length, insert.hole),
            (4.6, 3.9, 5.7, 4.0)
        );
        assert_eq!(insert.label(), "M3 insert");
        assert_eq!(insert.ops().len(), 3, "the core and two knurls");
    }

    #[test]
    fn the_short_length_is_a_choice() {
        let mut insert = Insert::new("M4", &Defaults::default());
        assert!(insert.apply(&PanelEvent::Choice {
            id: "short".into(),
            index: 1,
        }));
        assert_eq!(insert.length, 4.0);
        let insert = Insert::with_args(
            &json!({"size": "M5", "short": true, "thread": true}),
            &Defaults::default(),
        )
        .unwrap();
        assert_eq!(insert.length, 5.8);
        let roles: Vec<_> = insert.ops().iter().map(|op| op.boolean_op()).collect();
        assert_eq!(
            roles,
            [
                Some(BooleanOp::NewSolid),
                Some(BooleanOp::Cut),
                Some(BooleanOp::Fuse),
                Some(BooleanOp::Fuse)
            ],
            "the thread is cut before the knurls go on"
        );
        assert!(Insert::with_args(&json!({"size": "M10"}), &Defaults::default()).is_err());
        // M8 is made in one length, so no choice is offered.
        let m8 = Insert::new("M8", &Defaults::default());
        assert_eq!(
            (m8.outer, m8.pilot, m8.length, m8.hole),
            (10.2, 9.5, 12.7, 9.6)
        );
        let ctx = Ctx {
            feature: "f",
            focus: None,
        };
        assert!(
            !m8.panel(&ctx)
                .iter()
                .any(|w| matches!(w, Widget::Choice { id, .. } if id == "short"))
        );
    }
}
