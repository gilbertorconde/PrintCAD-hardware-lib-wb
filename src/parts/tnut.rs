//! T-slot nuts for the extrusion series, as their maker draws them:
//! sliding, drop-in, spring-ball, twist and roll-in. A nut sits as it
//! does in a slot that runs along Y with its opening up: its bottom on
//! the cavity's floor at `z = 0`, its top under the lips.

use printcad_bench_sdk::Value;
use printcad_bench_sdk::api::kernel_api::{
    BooleanOp, Placement, PrimitiveKind, ProfilePlane, SolidOp,
};
use printcad_bench_sdk::api::{DiagramStroke, PanelEvent, Parameter, Widget};
use serde::{Deserialize, Serialize};

use super::{
    Ctx, Defaults, Family, Part, arg_bool, arg_f64, arg_str, choice, fmt, group, index_of, length,
    note, number, toggle,
};
use crate::diagram::Sketch;
use crate::geom;
pub use crate::standards::TNutKind;
use crate::standards::{self, TNutRow, internal_minor};

impl TNutKind {
    pub const ALL: [TNutKind; 5] = [
        TNutKind::Sliding,
        TNutKind::DropIn,
        TNutKind::SpringBall,
        TNutKind::Twist,
        TNutKind::RollIn,
    ];

    pub fn name(self) -> &'static str {
        match self {
            TNutKind::Sliding => "Sliding",
            TNutKind::DropIn => "Drop-in",
            TNutKind::SpringBall => "Drop-in, spring ball",
            TNutKind::Twist => "Twist",
            TNutKind::RollIn => "Roll-in, set screw",
        }
    }

    fn short(self) -> &'static str {
        match self {
            TNutKind::Sliding => "sliding T-nut",
            TNutKind::DropIn => "drop-in T-nut",
            TNutKind::SpringBall => "spring T-nut",
            TNutKind::Twist => "twist T-nut",
            TNutKind::RollIn => "roll-in T-nut",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            TNutKind::Sliding => "tnut-sliding",
            TNutKind::DropIn => "tnut-drop-in",
            TNutKind::SpringBall => "tnut-spring",
            TNutKind::Twist => "tnut-twist",
            TNutKind::RollIn => "tnut-roll-in",
        }
    }

    pub fn tool(self) -> &'static str {
        match self {
            TNutKind::Sliding => "sliding_tnut",
            TNutKind::DropIn => "drop_in_tnut",
            TNutKind::SpringBall => "spring_tnut",
            TNutKind::Twist => "twist_tnut",
            TNutKind::RollIn => "roll_in_tnut",
        }
    }

    pub fn named(name: &str) -> Option<TNutKind> {
        let key = name.to_lowercase().replace(['-', ' '], "_");
        TNutKind::ALL.into_iter().find(|k| {
            k.tool() == key
                || k.tool().trim_end_matches("_tnut") == key
                || k.name().eq_ignore_ascii_case(name)
                || (key == "spring_ball" && *k == TNutKind::SpringBall)
                || (key == "dropin" && *k == TNutKind::DropIn)
                || (key == "rollin" && *k == TNutKind::RollIn)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TNut {
    pub kind: TNutKind,
    pub series: u32,
    pub size: String,
    pub d: f64,
    pub pitch: f64,
    /// Along the slot.
    pub length: f64,
    /// Across the slot: under the lips, and on the floor.
    pub top: f64,
    pub bottom: f64,
    pub thick: f64,
    /// How far down from the top the sides run straight before they turn
    /// in to the bottom.
    pub straight: f64,
    /// Where the thread sits from one end; 0 for the middle.
    pub thread_at: f64,
    #[serde(default)]
    pub custom: bool,
    #[serde(default)]
    pub thread: bool,
}

/// The twist nut's ends, from square.
const TWIST_DEG: f64 = 15.0;

impl TNut {
    pub fn new(kind: TNutKind, series: u32, size: &str, defaults: &Defaults) -> TNut {
        let mut nut = TNut {
            kind,
            series,
            size: size.into(),
            d: 0.0,
            pitch: 0.0,
            length: 0.0,
            top: 0.0,
            bottom: 0.0,
            thick: 0.0,
            straight: 0.0,
            thread_at: 0.0,
            custom: false,
            thread: defaults.thread,
        };
        nut.refill();
        nut
    }

    pub fn row(&self) -> &'static TNutRow {
        standards::tnut_row(self.series, self.kind)
            .or_else(|| standards::tnut_row(20, self.kind))
            .unwrap_or(&standards::T_NUTS[0])
    }

    fn sizes(&self) -> Vec<&'static str> {
        self.row().sizes.to_vec()
    }

    pub fn refill(&mut self) {
        let row = *self.row();
        self.series = row.series;
        if !row.sizes.iter().any(|s| s.eq_ignore_ascii_case(&self.size)) {
            // The nearest size the nut is made in.
            let want = standards::metric(&self.size)
                .map(|(d, _)| d)
                .unwrap_or(self.d);
            self.size = row
                .sizes
                .iter()
                .min_by(|a, b| {
                    let da = standards::metric(a).map_or(f64::MAX, |(d, _)| (d - want).abs());
                    let db = standards::metric(b).map_or(f64::MAX, |(d, _)| (d - want).abs());
                    da.total_cmp(&db)
                })
                .map_or(row.sizes[0], |s| *s)
                .into();
        }
        let (d, pitch) = standards::metric(&self.size).unwrap_or((5.0, 0.8));
        self.d = d;
        self.pitch = pitch;
        self.length = row.length;
        self.top = row.top;
        self.bottom = row.bottom;
        self.thick = row.thick;
        self.straight = row.straight;
        self.thread_at = row.thread_at;
    }

    fn minor(&self) -> f64 {
        internal_minor(self.d, self.pitch)
    }

    /// Where the thread sits along the slot.
    fn thread_y(&self) -> f64 {
        if self.thread_at > 0.0 {
            self.length / 2.0 - self.thread_at
        } else {
            0.0
        }
    }

    /// The set screw's hole, a roll-in nut's: M4, in from the end
    /// opposite the thread.
    fn set_screw_y(&self) -> f64 {
        -(self.length / 2.0 - 3.5)
    }

    /// The sprung ball: its diameter, how far it stands proud of the
    /// bottom, and where it sits along the slot.
    fn ball(&self) -> (f64, f64, f64) {
        let scale = self.series as f64 / 20.0;
        let proud = match self.series {
            30 => 0.5,
            _ => 0.7,
        };
        (3.0 * scale, proud, -(self.length / 2.0 - 4.5 * scale))
    }

    /// The section across the slot, in `(x, z)`, counter-clockwise.
    pub fn section(&self) -> Vec<[f64; 2]> {
        let (b, w, t, e) = (self.bottom / 2.0, self.top / 2.0, self.thick, self.straight);
        vec![
            [-b, 0.0],
            [b, 0.0],
            [w, t - e],
            [w, t],
            [-w, t],
            [-w, t - e],
        ]
    }

    /// The plan, in `(x, y)`: a rectangle, or a twist nut's parallelogram
    /// with its ends cut at 15°.
    pub fn plan(&self) -> Vec<[f64; 2]> {
        let (w, l) = (self.top / 2.0, self.length / 2.0);
        match self.kind {
            TNutKind::Twist => {
                let k = w * TWIST_DEG.to_radians().tan();
                vec![[-w, -l + k], [w, -l - k], [w, l - k], [-w, l + k]]
            }
            _ => vec![[-w, -l], [w, -l], [w, l], [-w, l]],
        }
    }
}

impl Part for TNut {
    const FAMILY: Family = Family::TNut;

    fn label(&self) -> String {
        format!(
            "{} {} ({} series)",
            self.size,
            self.kind.short(),
            self.series
        )
    }

    fn icon(&self) -> &'static str {
        self.kind.icon()
    }

    fn problem(&self) -> Option<String> {
        if self.d <= 0.0 || self.pitch <= 0.0 || self.length <= 0.0 || self.thick <= 0.0 {
            return Some("The thread, the length and the thickness must be more than 0.".into());
        }
        if self.bottom <= 0.0 || self.top < self.bottom {
            return Some("The top must be as wide as the bottom or wider.".into());
        }
        if self.straight <= 0.0 || self.straight >= self.thick {
            return Some("The straight part of the sides must lie within the thickness.".into());
        }
        if self.d >= self.bottom - 0.4 {
            return Some("The thread is wider than the nut's bottom.".into());
        }
        if self.thread_at > 0.0
            && (self.thread_at >= self.length - self.d / 2.0 || self.thread_at <= self.d / 2.0)
        {
            return Some("The thread must lie within the nut's length.".into());
        }
        if self.kind == TNutKind::RollIn && self.length < 12.0 {
            return Some("A roll-in nut needs room for its set screw: 12 mm or longer.".into());
        }
        if self.minor() <= 0.0 {
            return Some("The pitch is too coarse for the thread's diameter.".into());
        }
        None
    }

    fn axis(&self) -> ([f64; 3], [f64; 3]) {
        // A spring nut's ball stands proud of its bottom.
        let below = if self.kind == TNutKind::SpringBall {
            self.ball().1
        } else {
            0.0
        };
        super::z_axis(-below, self.thick)
    }

    fn ops(&self) -> Vec<SolidOp> {
        let l = self.length;
        // The section runs along the slot: drawn in XZ and extruded along
        // -Y from y = l / 2, so the nut is centred on the origin; a twist
        // nut's is cut to its parallelogram.
        let run = if self.kind == TNutKind::Twist {
            l + 2.0 * self.top
        } else {
            l
        };
        let plane = ProfilePlane {
            origin: [0.0, run / 2.0, 0.0],
            x_axis: [1.0, 0.0, 0.0],
            y_axis: [0.0, 0.0, 1.0],
            normal: [0.0, -1.0, 0.0],
        };
        let mut ops = vec![geom::extrude(
            plane,
            vec![geom::polygon(&self.section())],
            run,
            BooleanOp::NewSolid,
        )];
        if self.kind == TNutKind::Twist {
            ops.push(geom::prism(
                &self.plan(),
                Vec::new(),
                -1.0,
                self.thick + 2.0,
                BooleanOp::Common,
            ));
        }
        // The thread, cut into the plain body before anything else goes on.
        let thread_c = [0.0, self.thread_y()];
        ops.push(geom::extrude(
            geom::xy_down(self.thick + 1.0),
            vec![vec![geom::circle_at(thread_c, self.minor())]],
            self.thick + 2.0,
            BooleanOp::Cut,
        ));
        if self.thread {
            ops.push(geom::internal_thread_at(
                self.d,
                self.minor(),
                self.pitch,
                self.thick,
                self.thick,
                thread_c,
            ));
        }
        if self.kind == TNutKind::RollIn {
            ops.push(geom::extrude(
                geom::xy_down(self.thick + 1.0),
                vec![vec![geom::circle_at([0.0, self.set_screw_y()], 3.3)]],
                self.thick + 2.0,
                BooleanOp::Cut,
            ));
        }
        if self.kind == TNutKind::SpringBall {
            let (d, proud, y) = self.ball();
            ops.push(SolidOp::Primitive {
                kind: PrimitiveKind::Sphere {
                    radius: d / 2.0,
                    angle1_deg: -90.0,
                    angle2_deg: 90.0,
                    angle3_deg: 360.0,
                },
                placement: Placement {
                    origin: [0.0, y, d / 2.0 - proud],
                    ..Default::default()
                },
                op: BooleanOp::Fuse,
            });
        }
        ops
    }

    fn panel(&self, ctx: &Ctx) -> Vec<Widget> {
        let kinds: Vec<&str> = TNutKind::ALL.iter().map(|k| k.name()).collect();
        let series: Vec<String> = standards::EXTRUSIONS
            .iter()
            .map(|s| format!("{} series, slot {}", s.cell, fmt(s.opening)))
            .collect();
        let sizes = self.sizes();
        let mut widgets = vec![
            self.drawing(ctx),
            choice(
                "kind",
                "Nut",
                &kinds,
                TNutKind::ALL
                    .iter()
                    .position(|k| *k == self.kind)
                    .unwrap_or(0),
            ),
            choice(
                "series",
                "Extrusion",
                &series,
                standards::EXTRUSIONS
                    .iter()
                    .position(|s| s.cell == self.series)
                    .unwrap_or(0),
            ),
            choice("size", "Thread", &sizes, index_of(&sizes, &self.size)),
        ];
        let mut dims = vec![toggle("custom", "Custom dimensions", self.custom)];
        if self.custom {
            dims.push(number(ctx, "d", "Thread diameter", self.d, 0.1, 2));
            dims.push(number(ctx, "pitch", "Pitch", self.pitch, 0.05, 2));
            dims.push(number(
                ctx,
                "length",
                "Length along slot",
                self.length,
                0.1,
                2,
            ));
            dims.push(number(ctx, "top", "Width under the lips", self.top, 0.1, 2));
            dims.push(number(
                ctx,
                "bottom",
                "Width on the floor",
                self.bottom,
                0.1,
                2,
            ));
            dims.push(number(ctx, "thick", "Thickness", self.thick, 0.1, 2));
            dims.push(number(
                ctx,
                "straight",
                "Straight sides",
                self.straight,
                0.1,
                2,
            ));
            dims.push(number(
                ctx,
                "thread_at",
                "Thread from end (0: middle)",
                self.thread_at,
                0.0,
                2,
            ));
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
            length("length", "Length"),
            length("top", "Width under the lips"),
            length("bottom", "Width on the floor"),
            length("thick", "Thickness"),
            length("straight", "Straight sides"),
            length("thread_at", "Thread from end"),
        ]
    }

    fn apply(&mut self, event: &PanelEvent) -> bool {
        match event {
            PanelEvent::Choice { id, index } => match id.as_str() {
                "kind" => {
                    self.kind = TNutKind::ALL
                        .get(*index)
                        .copied()
                        .unwrap_or(TNutKind::Sliding);
                    self.refill();
                }
                "series" => {
                    self.series = standards::EXTRUSIONS.get(*index).map_or(20, |s| s.cell);
                    self.refill();
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
                "length" => self.length = *value,
                "top" => self.top = *value,
                "bottom" => self.bottom = *value,
                "thick" => self.thick = *value,
                "straight" => self.straight = *value,
                "thread_at" => self.thread_at = *value,
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
            None => TNutKind::Sliding,
            Some(name) => TNutKind::named(name).ok_or_else(|| {
                format!(
                    "`{name}` is not a T-slot nut: {}",
                    TNutKind::ALL.map(|k| k.tool()).join(", ")
                )
            })?,
        };
        let series = arg_f64(args, "series").map_or(defaults.series, |s| s as u32);
        if standards::extrusion(series).is_none() {
            return Err(format!("`{series}` is not a series: 20, 30 or 40"));
        }
        let size = arg_str(args, "size").unwrap_or(&defaults.size);
        let mut nut = TNut::new(kind, series, size, defaults);
        if arg_str(args, "size").is_some() && !nut.size.eq_ignore_ascii_case(size) {
            return Err(format!(
                "a {} series {} is not made in {size}: {}",
                series,
                kind.short(),
                nut.sizes().join(", ")
            ));
        }
        for (key, slot) in [
            ("d", &mut nut.d),
            ("pitch", &mut nut.pitch),
            ("length", &mut nut.length),
            ("top", &mut nut.top),
            ("bottom", &mut nut.bottom),
            ("thick", &mut nut.thick),
            ("straight", &mut nut.straight),
            ("thread_at", &mut nut.thread_at),
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

impl TNut {
    /// The section across the slot, drawn in its slot, with the plan
    /// beside it.
    fn drawing(&self, ctx: &Ctx) -> Widget {
        let mut s = Sketch::new(ctx.focus);
        let series = standards::extrusion(self.series).unwrap_or(&standards::EXTRUSIONS[0]);
        let (b, w, t, e) = (self.bottom / 2.0, self.top / 2.0, self.thick, self.straight);
        let off = Sketch::standoff(self.length.max(self.top));
        // The slot around it: the floor at z = 0, the lips above.
        let floor = (series.cavity - 2.0 * (series.depth - series.shoulder)) / 2.0;
        let (cav, o) = (series.cavity / 2.0, series.opening / 2.0);
        let (lip_z, shoulder_z, top_z) = (
            series.depth - series.lip,
            series.depth - series.shoulder,
            series.depth,
        );
        let reach = cav + series.lip * 1.5;
        for sign in [-1.0, 1.0] {
            s.line(
                &[
                    [sign * reach, top_z],
                    [sign * o, top_z],
                    [sign * o, lip_z],
                    [sign * cav, lip_z],
                    [sign * cav, shoulder_z],
                    [sign * floor, 0.0],
                    [0.0, 0.0],
                ],
                DiagramStroke::Thin,
            );
        }
        // The nut's section, shaded.
        s.poly(&self.section(), DiagramStroke::Outline, true);
        s.hidden(&[[-self.d / 2.0, 0.0], [-self.d / 2.0, t]]);
        s.hidden(&[[self.d / 2.0, 0.0], [self.d / 2.0, t]]);
        s.width("top", -w, w, t, off * 0.5, fmt(self.top));
        s.width("bottom", -b, b, 0.0, -off, fmt(self.bottom));
        s.height("thick", -reach, 0.0, t, off, format!("T {}", fmt(t)));
        s.height("straight", reach, t - e, t, -off * 0.6, fmt(e));
        // The plan, beside: the thread, a set screw, a ball.
        let x0 = reach + off * 2.2 + self.top / 2.0;
        let plan: Vec<[f64; 2]> = self.plan().iter().map(|p| [p[0] + x0, p[1]]).collect();
        s.poly(&plan, DiagramStroke::Outline, false);
        s.circle([x0, self.thread_y()], self.d, DiagramStroke::Outline, false);
        if self.kind == TNutKind::RollIn {
            s.circle([x0, self.set_screw_y()], 3.3, DiagramStroke::Outline, false);
        }
        if self.kind == TNutKind::SpringBall {
            let (d, _, y) = self.ball();
            s.circle([x0, y], d, DiagramStroke::Hidden, false);
        }
        let l = self.length / 2.0;
        s.height(
            "length",
            x0 + w,
            -l,
            l,
            -off,
            format!("L {}", fmt(self.length)),
        );
        s.callout(
            "d",
            [x0 + self.d / 2.0, self.thread_y()],
            [x0 + w + off * 1.6, l + off * 0.8],
            format!("{} × {}", self.size, fmt(self.pitch)),
        );
        if self.thread_at > 0.0 {
            s.height(
                "thread_at",
                x0 - w,
                self.thread_y(),
                l,
                off * 0.8,
                fmt(self.thread_at),
            );
        }
        s.finish("tnut")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printcad_bench_sdk::json;

    fn nut(kind: TNutKind, series: u32) -> TNut {
        TNut::new(kind, series, "M5", &Defaults::default())
    }

    #[test]
    fn a_20_series_sliding_nut_is_its_makers() {
        let n = nut(TNutKind::Sliding, 20);
        assert_eq!(
            (n.length, n.top, n.bottom, n.thick, n.straight),
            (10.0, 10.0, 5.8, 4.1, 3.3)
        );
        assert_eq!(n.label(), "M5 sliding T-nut (20 series)");
        assert_eq!(n.problem(), None);
        let roles: Vec<_> = n.ops().iter().map(|op| op.boolean_op()).collect();
        assert_eq!(roles, [Some(BooleanOp::NewSolid), Some(BooleanOp::Cut)]);
    }

    #[test]
    fn every_kind_of_every_series_builds_and_fits_its_slot() {
        for series in standards::EXTRUSIONS {
            for kind in TNutKind::ALL {
                let n = TNut::new(kind, series.cell, "M4", &Defaults::default());
                assert_eq!(n.problem(), None, "{} {:?}", series.cell, kind);
                assert!(n.top < series.cavity, "{} {:?}", series.cell, kind);
                assert!(
                    n.thick <= series.depth - series.lip + 0.11,
                    "{} {:?}",
                    series.cell,
                    kind
                );
                assert!(!n.ops().is_empty());
                let ctx = Ctx {
                    feature: "f",
                    focus: Some("thick"),
                };
                assert!(matches!(
                    n.panel(&ctx).first(),
                    Some(Widget::Diagram { .. })
                ));
            }
        }
    }

    #[test]
    fn a_twist_nut_is_a_parallelogram_cut_from_its_section() {
        let n = nut(TNutKind::Twist, 20);
        assert_eq!((n.top, n.length), (6.0, 10.0));
        let plan = n.plan();
        assert!((plan[1][1] - plan[0][1]).abs() > 1.0, "its ends lean 15°");
        let roles: Vec<_> = n.ops().iter().map(|op| op.boolean_op()).collect();
        assert_eq!(
            roles,
            [
                Some(BooleanOp::NewSolid),
                Some(BooleanOp::Common),
                Some(BooleanOp::Cut)
            ]
        );
    }

    #[test]
    fn a_roll_in_nut_has_its_set_screw_and_a_spring_nut_its_ball() {
        let n = nut(TNutKind::RollIn, 20);
        assert_eq!(n.ops().len(), 3);
        assert!((n.thread_y() - 2.0).abs() < 1e-9, "6.5 from one end of 17");
        let n = nut(TNutKind::SpringBall, 30);
        assert!(matches!(n.ops().last(), Some(SolidOp::Primitive { .. })));
    }

    #[test]
    fn sizes_follow_the_series_and_a_command_names_its_nut() {
        let mut n = nut(TNutKind::Sliding, 20);
        assert_eq!(n.sizes(), ["M3", "M4", "M5"]);
        assert!(n.apply(&PanelEvent::Choice {
            id: "series".into(),
            index: 2,
        }));
        assert_eq!((n.series, n.size.as_str(), n.top), (40, "M5", 17.0));
        let n = TNut::with_args(
            &json!({"kind": "spring_ball", "series": 30, "size": "M6"}),
            &Defaults::default(),
        )
        .unwrap();
        assert_eq!((n.kind, n.length, n.d), (TNutKind::SpringBall, 17.0, 6.0));
        assert!(
            TNut::with_args(
                &json!({"kind": "twist", "series": 20, "size": "M6"}),
                &Defaults::default()
            )
            .is_err()
        );
        assert!(TNut::with_args(&json!({"kind": "hammer"}), &Defaults::default()).is_err());
    }
}
