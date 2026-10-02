//! Threaded standoffs and spacers, hex or round, on `z = 0` with their
//! axis up Z: female at both ends, or with a stud on top; bored for a
//! clearance hole, a thread, or a heat-set insert at each end.

use printcad_bench_sdk::Value;
use printcad_bench_sdk::api::kernel_api::{BooleanOp, ProfileSegment, SolidOp};
use printcad_bench_sdk::api::{DiagramStroke, PanelEvent, Parameter, Widget};
use serde::{Deserialize, Serialize};

use super::{
    Ctx, Defaults, Family, Part, arg_bool, arg_f64, arg_str, choice, default_length, fmt, group,
    index_of, length, note, number, toggle,
};
use crate::diagram::Sketch;
use crate::geom::{self, across_corners, revolve};
use crate::standards::{self, StandoffRow, internal_minor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandoffShape {
    Hex,
    Round,
}

impl StandoffShape {
    pub const ALL: [StandoffShape; 2] = [StandoffShape::Hex, StandoffShape::Round];

    pub fn name(self) -> &'static str {
        match self {
            StandoffShape::Hex => "Hex",
            StandoffShape::Round => "Round",
        }
    }

    pub fn tool(self) -> &'static str {
        match self {
            StandoffShape::Hex => "hex",
            StandoffShape::Round => "round",
        }
    }
}

/// What the standoff is bored for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bore {
    /// A clearance hole through: a spacer.
    Clear,
    /// A thread.
    Thread,
    /// A heat-set insert's hole at each female end.
    Insert,
}

impl Bore {
    pub const ALL: [Bore; 3] = [Bore::Thread, Bore::Clear, Bore::Insert];

    pub fn name(self) -> &'static str {
        match self {
            Bore::Clear => "Clearance hole (spacer)",
            Bore::Thread => "Threaded",
            Bore::Insert => "Heat-set insert holes",
        }
    }

    pub fn tool(self) -> &'static str {
        match self {
            Bore::Clear => "clear",
            Bore::Thread => "thread",
            Bore::Insert => "insert",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Standoff {
    pub shape: StandoffShape,
    pub size: String,
    pub d: f64,
    pub pitch: f64,
    /// Across flats, or a round one's diameter.
    pub across: f64,
    pub length: f64,
    pub bore: Bore,
    /// A stud on top, this long; 0 for female at both ends.
    pub stud: f64,
    /// An insert's hole and its depth, at each female end.
    pub hole: f64,
    pub depth: f64,
    #[serde(default)]
    pub custom: bool,
    #[serde(default)]
    pub thread: bool,
}

impl Standoff {
    pub fn new(shape: StandoffShape, size: &str, defaults: &Defaults) -> Standoff {
        let mut standoff = Standoff {
            shape,
            size: size.into(),
            d: 0.0,
            pitch: 0.0,
            across: 0.0,
            length: 0.0,
            bore: Bore::Thread,
            stud: 0.0,
            hole: 0.0,
            depth: 0.0,
            custom: false,
            thread: defaults.thread,
        };
        standoff.refill();
        standoff.length = default_length(standoff.d);
        standoff
    }

    fn row(&self) -> StandoffRow {
        standards::STANDOFFS
            .iter()
            .find(|r| r.size.eq_ignore_ascii_case(&self.size))
            .or_else(|| {
                let want = standards::metric(&self.size)
                    .map(|(d, _)| d)
                    .unwrap_or(self.d);
                standards::STANDOFFS
                    .iter()
                    .min_by(|a, b| (a.d - want).abs().total_cmp(&(b.d - want).abs()))
            })
            .copied()
            .unwrap_or(standards::STANDOFFS[2])
    }

    fn sizes() -> Vec<&'static str> {
        standards::STANDOFFS.iter().map(|r| r.size).collect()
    }

    /// The stud a male-female one of this size carries.
    fn standard_stud(&self) -> f64 {
        self.row().stud
    }

    pub fn refill(&mut self) {
        let row = self.row();
        self.size = row.size.into();
        self.d = row.d;
        self.pitch = row.pitch;
        self.across = row.across;
        if self.stud > 0.0 {
            self.stud = row.stud;
        }
        self.refill_insert();
    }

    /// The insert's hole for the thread, from the inserts' table.
    fn refill_insert(&mut self) {
        match standards::insert_for(self.d) {
            Some(i) => {
                self.hole = i.hole;
                self.depth = i.length + 1.0;
            }
            None => {
                self.hole = ((1.35 * self.d) * 10.0).round() / 10.0;
                self.depth = ((1.6 * self.d) * 10.0).round() / 10.0;
            }
        }
    }

    /// Size the standoff `size`, a diameter off the table, in proportion.
    fn proportion(&mut self, size: &str, d: f64, pitch: f64) {
        self.size = size.trim().to_uppercase();
        self.d = d;
        self.pitch = pitch;
        self.across = ((1.6 * d + 0.5) * 10.0).round() / 10.0;
        if self.stud > 0.0 {
            self.stud = (2.0 * d).round();
        }
        self.refill_insert();
    }

    fn male(&self) -> bool {
        self.stud > 0.0
    }

    fn minor(&self) -> f64 {
        internal_minor(self.d, self.pitch)
    }

    /// The clearance hole a spacer takes: ISO 273's medium series.
    fn clearance(&self) -> f64 {
        standards::clearance(self.d).map_or(((1.1 * self.d) * 10.0).round() / 10.0, |h| h.medium)
    }

    /// The outline across, in `(x, y)`.
    fn outline(&self) -> Vec<[f64; 2]> {
        match self.shape {
            StandoffShape::Hex => geom::hexagon(self.across),
            StandoffShape::Round => geom::regular(48, self.across),
        }
    }

    /// How wide it is over all, corners and all.
    fn wide(&self) -> f64 {
        match self.shape {
            StandoffShape::Hex => across_corners(self.across),
            StandoffShape::Round => self.across,
        }
    }

    /// A threaded bore's depth from the bottom: through, or short of a
    /// stud's root by a diameter.
    fn thread_depth(&self) -> f64 {
        if self.male() {
            (self.length - self.d).max(0.0)
        } else {
            self.length
        }
    }

    /// What is bored, for a label.
    fn bore_text(&self) -> String {
        match self.bore {
            Bore::Clear => format!("Ø{} clearance hole through", fmt(self.clearance())),
            Bore::Thread if self.male() => format!(
                "{} thread {} deep below, a {} stud on top",
                self.size,
                fmt(self.thread_depth()),
                fmt(self.stud)
            ),
            Bore::Thread => format!("{} thread through", self.size),
            Bore::Insert if self.male() => format!(
                "Ø{} × {} insert hole below, a {} stud on top",
                fmt(self.hole),
                fmt(self.depth),
                fmt(self.stud)
            ),
            Bore::Insert => format!(
                "Ø{} × {} insert hole at each end",
                fmt(self.hole),
                fmt(self.depth)
            ),
        }
    }
}

impl Part for Standoff {
    const FAMILY: Family = Family::Standoff;

    fn label(&self) -> String {
        let what = match (self.bore, self.male()) {
            (Bore::Clear, _) => "spacer",
            (_, true) => "male-female standoff",
            (_, false) => "standoff",
        };
        format!(
            "{} × {} {} {what}",
            self.size,
            fmt(self.length),
            self.shape.tool()
        )
    }

    fn icon(&self) -> &'static str {
        "standoff"
    }

    fn problem(&self) -> Option<String> {
        if self.d <= 0.0 || self.pitch <= 0.0 || self.length <= 0.0 || self.across <= 0.0 {
            return Some("The thread, the width and the length must be more than 0.".into());
        }
        let wall = match self.bore {
            Bore::Clear => self.across - self.clearance(),
            Bore::Thread => self.across - self.d,
            Bore::Insert => self.across - self.hole,
        };
        if wall < 0.6 {
            return Some("The bore leaves less than 0.3 mm of wall: make it wider.".into());
        }
        if self.male() && self.bore == Bore::Clear {
            return Some("A spacer has no stud: bore it for a thread or an insert, or make it female at both ends.".into());
        }
        if self.male() && self.bore == Bore::Thread && self.thread_depth() < self.pitch {
            return Some("The standoff is too short for a thread under its stud.".into());
        }
        if self.bore == Bore::Insert {
            if self.hole <= self.d || self.depth <= 0.0 {
                return Some(
                    "The insert's hole must be wider than the thread and deeper than 0.".into(),
                );
            }
            let ends = if self.male() { 1.0 } else { 2.0 };
            if ends * self.depth > self.length - 0.5 {
                return Some("The standoff is too short for its insert holes.".into());
            }
        }
        if self.minor() <= 0.0 {
            return Some("The pitch is too coarse for the thread's diameter.".into());
        }
        None
    }

    fn axis(&self) -> ([f64; 3], [f64; 3]) {
        super::z_axis(0.0, self.length + self.stud)
    }

    fn ops(&self) -> Vec<SolidOp> {
        let (l, r) = (self.length, self.d / 2.0);
        let mut ops = Vec::new();
        // A stud first: a plain body of revolution its thread is cut into
        // before the body is fused on; it reaches a little into the body.
        if self.male() {
            let c = (0.6 * self.pitch).min(r / 2.0);
            let top = l + self.stud;
            ops.push(revolve(
                &[
                    [0.0, l - 0.5],
                    [r, l - 0.5],
                    [r, top - c],
                    [r - c, top],
                    [0.0, top],
                ],
                BooleanOp::NewSolid,
            ));
            if self.thread && self.stud > 3.0 * self.pitch {
                // Short of the body by two pitches (the groove runs a pitch
                // past its length), so it stays on the stud.
                ops.push(geom::external_thread(
                    self.d,
                    self.pitch,
                    top,
                    self.stud - 2.0 * self.pitch,
                    true,
                ));
            }
        }
        // The body, with what goes through it as a hole of its outline.
        let through: Vec<ProfileSegment> = match self.bore {
            Bore::Clear => vec![geom::circle(self.clearance())],
            Bore::Thread if !self.male() => vec![geom::circle(self.minor())],
            Bore::Insert if !self.male() => vec![geom::circle(self.d + 0.5)],
            _ => Vec::new(),
        };
        let role = if self.male() {
            BooleanOp::Fuse
        } else {
            BooleanOp::NewSolid
        };
        ops.push(geom::prism(&self.outline(), through, 0.0, l, role));
        // The holes that stop short.
        match self.bore {
            Bore::Thread if self.male() => {
                ops.push(geom::extrude(
                    geom::xy(-1.0),
                    vec![vec![geom::circle(self.minor())]],
                    self.thread_depth() + 1.0,
                    BooleanOp::Cut,
                ));
            }
            Bore::Insert => {
                ops.push(geom::extrude(
                    geom::xy(-1.0),
                    vec![vec![geom::circle(self.hole)]],
                    self.depth + 1.0,
                    BooleanOp::Cut,
                ));
                if !self.male() {
                    ops.push(geom::extrude(
                        geom::xy(l - self.depth),
                        vec![vec![geom::circle(self.hole)]],
                        self.depth + 1.0,
                        BooleanOp::Cut,
                    ));
                }
            }
            _ => {}
        }
        // The thread in the bore, clear of the stud's root.
        if self.thread && self.bore == Bore::Thread {
            let depth = self.thread_depth();
            ops.push(geom::internal_thread_at(
                self.d,
                self.minor(),
                self.pitch,
                depth,
                depth,
                [0.0, 0.0],
            ));
        }
        ops
    }

    fn panel(&self, ctx: &Ctx) -> Vec<Widget> {
        let shapes: Vec<&str> = StandoffShape::ALL.iter().map(|s| s.name()).collect();
        let bores: Vec<&str> = Bore::ALL.iter().map(|b| b.name()).collect();
        let sizes = Self::sizes();
        let mut widgets = vec![
            self.drawing(ctx),
            choice(
                "shape",
                "Shape",
                &shapes,
                StandoffShape::ALL
                    .iter()
                    .position(|s| *s == self.shape)
                    .unwrap_or(0),
            ),
            choice("size", "Thread", &sizes, index_of(&sizes, &self.size)),
            number(ctx, "length", "Length", self.length, 0.1, 1),
            choice(
                "ends",
                "Ends",
                &["Female at both ends", "Stud on top (male-female)"],
                usize::from(self.male()),
            ),
            choice(
                "bore",
                "Bore",
                &bores,
                Bore::ALL.iter().position(|b| *b == self.bore).unwrap_or(0),
            ),
        ];
        if self.male() {
            widgets.push(number(ctx, "stud", "Stud length", self.stud, 0.1, 1));
        }
        widgets.push(super::text(self.bore_text()));
        let mut dims = vec![toggle("custom", "Custom dimensions", self.custom)];
        if self.custom {
            dims.push(number(ctx, "d", "Thread diameter", self.d, 0.1, 2));
            dims.push(number(ctx, "pitch", "Pitch", self.pitch, 0.05, 2));
            let width = match self.shape {
                StandoffShape::Hex => "Across flats",
                StandoffShape::Round => "Diameter",
            };
            dims.push(number(ctx, "across", width, self.across, 0.1, 2));
            if self.bore == Bore::Insert {
                dims.push(number(ctx, "hole", "Insert hole", self.hole, 0.1, 2));
                dims.push(number(
                    ctx,
                    "depth",
                    "Insert hole depth",
                    self.depth,
                    0.1,
                    2,
                ));
            }
        }
        widgets.push(group("Dimensions", self.custom, dims));
        if self.bore == Bore::Thread || self.male() {
            let mut options = vec![toggle("thread", "Modelled thread", self.thread)];
            if self.thread {
                options.push(super::text("A modelled thread takes the kernel a while."));
            }
            widgets.push(group("Options", true, options));
        }
        widgets.extend(note(self.problem()));
        widgets
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            length("length", "Length"),
            length("d", "Thread diameter"),
            length("pitch", "Pitch"),
            length("across", "Across"),
            length("stud", "Stud length"),
            length("hole", "Insert hole"),
            length("depth", "Insert hole depth"),
        ]
    }

    fn apply(&mut self, event: &PanelEvent) -> bool {
        match event {
            PanelEvent::Choice { id, index } => match id.as_str() {
                "shape" => {
                    self.shape = StandoffShape::ALL
                        .get(*index)
                        .copied()
                        .unwrap_or(StandoffShape::Hex);
                }
                "size" => {
                    if let Some(size) = Self::sizes().get(*index) {
                        self.size = (*size).into();
                        self.refill();
                    }
                }
                "ends" => {
                    self.stud = if *index == 1 {
                        self.standard_stud()
                    } else {
                        0.0
                    };
                    if self.male() && self.bore == Bore::Clear {
                        self.bore = Bore::Thread;
                    }
                }
                "bore" => {
                    self.bore = Bore::ALL.get(*index).copied().unwrap_or(Bore::Thread);
                }
                _ => return false,
            },
            PanelEvent::Number { id, value } => match id.as_str() {
                "length" => self.length = *value,
                "d" => self.d = *value,
                "pitch" => self.pitch = *value,
                "across" => self.across = *value,
                "stud" => self.stud = *value,
                "hole" => self.hole = *value,
                "depth" => self.depth = *value,
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
        let shape = match arg_str(args, "shape") {
            None => StandoffShape::Hex,
            Some(name) => StandoffShape::ALL
                .into_iter()
                .find(|s| {
                    s.tool().eq_ignore_ascii_case(name) || s.name().eq_ignore_ascii_case(name)
                })
                .ok_or_else(|| format!("`{name}` is not a standoff shape: hex, round"))?,
        };
        let size = arg_str(args, "size").unwrap_or(&defaults.size);
        let mut standoff = Standoff::new(shape, size, defaults);
        if let Some(ends) = arg_str(args, "ends") {
            standoff.stud = match ends.to_lowercase().as_str() {
                "ff" | "female" | "female_female" => 0.0,
                "mf" | "male" | "male_female" => standoff.standard_stud(),
                other => return Err(format!("`{other}` is not an end: `ff` or `mf`")),
            };
        }
        if arg_str(args, "size").is_some() && !standoff.size.eq_ignore_ascii_case(size) {
            let (d, pitch) = standards::metric_any(size)
                .ok_or_else(|| format!("no standoff is made in `{size}`"))?;
            standoff.proportion(size, d, pitch);
            standoff.length = default_length(d);
        }
        if let Some(bore) = arg_str(args, "bore") {
            standoff.bore = Bore::ALL
                .into_iter()
                .find(|b| b.tool().eq_ignore_ascii_case(bore))
                .ok_or_else(|| format!("`{bore}` is not a bore: clear, thread, insert"))?;
        }
        if let Some(l) = arg_f64(args, "length") {
            standoff.length = l;
        }
        if let Some(stud) = arg_f64(args, "stud") {
            standoff.stud = stud;
        }
        for (key, slot) in [
            ("d", &mut standoff.d),
            ("pitch", &mut standoff.pitch),
            ("across", &mut standoff.across),
            ("hole", &mut standoff.hole),
            ("depth", &mut standoff.depth),
        ] {
            if let Some(v) = arg_f64(args, key) {
                *slot = v;
                standoff.custom = true;
            }
        }
        if let Some(on) = arg_bool(args, "thread") {
            standoff.thread = on;
        }
        Ok(standoff)
    }
}

impl Standoff {
    /// The side view: the body, its bores behind the surface, the stud.
    fn drawing(&self, ctx: &Ctx) -> Widget {
        let mut s = Sketch::new(ctx.focus);
        let (l, w) = (self.length, self.wide() / 2.0);
        let r = self.d / 2.0;
        let top = l + self.stud;
        let off = Sketch::standoff(self.wide().max(top) * 0.7);
        s.rect([-w, 0.0], [w, l]);
        if self.shape == StandoffShape::Hex {
            let a = w / 2.0;
            s.line(&[[-a, 0.0], [-a, l]], DiagramStroke::Outline);
            s.line(&[[a, 0.0], [a, l]], DiagramStroke::Outline);
        }
        match self.bore {
            Bore::Clear => {
                let c = self.clearance() / 2.0;
                s.hidden(&[[-c, 0.0], [-c, l]]);
                s.hidden(&[[c, 0.0], [c, l]]);
            }
            Bore::Thread => {
                let depth = self.thread_depth();
                s.hidden(&[[-r, 0.0], [-r, depth], [r, depth], [r, 0.0]]);
            }
            Bore::Insert => {
                let h = self.hole / 2.0;
                s.hidden(&[[-h, 0.0], [-h, self.depth], [h, self.depth], [h, 0.0]]);
                if !self.male() {
                    s.hidden(&[[-h, l], [-h, l - self.depth], [h, l - self.depth], [h, l]]);
                    s.hidden(&[[-r - 0.25, self.depth], [-r - 0.25, l - self.depth]]);
                    s.hidden(&[[r + 0.25, self.depth], [r + 0.25, l - self.depth]]);
                }
                s.height("depth", w, 0.0, self.depth, -off * 0.6, fmt(self.depth));
            }
        }
        if self.male() {
            s.rect([-r, l], [r, top]);
            s.height("stud", r, l, top, -off * 0.6, fmt(self.stud));
        }
        s.width(
            "across",
            -w,
            w,
            l,
            off * if self.male() { 1.0 } else { 0.5 },
            format!(
                "{} {}",
                if self.shape == StandoffShape::Hex {
                    "AF"
                } else {
                    "Ø"
                },
                fmt(self.across)
            ),
        );
        s.height("length", -w, 0.0, l, off, format!("L {}", fmt(l)));
        s.axis(0.0, -off * 0.4, top + off * 0.4);
        s.width(
            "d",
            -r,
            r,
            0.0,
            -off,
            format!("{} × {}", self.size, fmt(self.pitch)),
        );
        s.finish("standoff")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printcad_bench_sdk::json;

    #[test]
    fn a_standoff_is_sized_by_its_thread() {
        let s = Standoff::new(StandoffShape::Hex, "M3", &Defaults::default());
        assert_eq!(
            (s.across, s.length, s.stud, s.hole, s.depth),
            (5.5, 10.0, 0.0, 4.0, 6.7)
        );
        assert_eq!(s.label(), "M3 × 10 hex standoff");
        assert_eq!(s.problem(), None);
        assert_eq!(s.ops().len(), 1, "a prism with the bore through it");
    }

    #[test]
    fn a_male_female_standoff_has_a_stud_over_a_blind_thread() {
        let mut s = Standoff::new(StandoffShape::Hex, "M3", &Defaults::default());
        assert!(s.apply(&PanelEvent::Choice {
            id: "ends".into(),
            index: 1,
        }));
        assert_eq!(s.stud, 6.0);
        assert_eq!(s.thread_depth(), 7.0);
        assert_eq!(s.label(), "M3 × 10 hex male-female standoff");
        let roles: Vec<_> = s.ops().iter().map(|op| op.boolean_op()).collect();
        assert_eq!(
            roles,
            [
                Some(BooleanOp::NewSolid),
                Some(BooleanOp::Fuse),
                Some(BooleanOp::Cut)
            ],
            "the stud, the body on it, the bore"
        );
        s.thread = true;
        let roles: Vec<_> = s.ops().iter().map(|op| op.boolean_op()).collect();
        assert_eq!(
            roles,
            [
                Some(BooleanOp::NewSolid),
                Some(BooleanOp::Cut),
                Some(BooleanOp::Fuse),
                Some(BooleanOp::Cut),
                Some(BooleanOp::Cut)
            ],
            "the stud's thread before the body goes on; the bore's after"
        );
        s.bore = Bore::Clear;
        assert!(s.problem().is_some(), "a spacer has no stud");
    }

    #[test]
    fn insert_holes_sit_at_each_end_and_need_the_length() {
        let mut s = Standoff::new(StandoffShape::Round, "M4", &Defaults::default());
        s.bore = Bore::Insert;
        assert_eq!((s.hole, s.depth), (5.6, 9.1));
        assert!(s.problem().is_some(), "12 long will not take two 9.1 holes");
        s.length = 25.0;
        assert_eq!(s.problem(), None);
        assert_eq!(s.ops().len(), 3);
    }

    #[test]
    fn a_command_names_its_standoff() {
        let d = Defaults::default();
        let s = Standoff::with_args(
            &json!({"shape": "round", "size": "M5", "length": 20, "ends": "mf", "bore": "insert"}),
            &d,
        )
        .unwrap();
        assert_eq!(
            (s.shape, s.across, s.length, s.stud, s.bore),
            (StandoffShape::Round, 8.0, 20.0, 10.0, Bore::Insert)
        );
        assert_eq!(s.problem(), None);
        let odd = Standoff::with_args(&json!({"size": "M7"}), &d).unwrap();
        assert_eq!(
            (odd.size.as_str(), odd.across, odd.length),
            ("M7", 11.7, 25.0)
        );
        assert!(Standoff::with_args(&json!({"bore": "slot"}), &d).is_err());
        assert!(Standoff::with_args(&json!({"ends": "mm"}), &d).is_err());
    }
}
