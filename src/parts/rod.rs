//! Rods: smooth shafts, threaded rods, lead screws and dowel pins, from
//! `z = 0` up Z.

use printcad_bench_sdk::Value;
use printcad_bench_sdk::api::kernel_api::{BooleanOp, SolidOp};
use printcad_bench_sdk::api::{DiagramStroke, PanelEvent, Parameter, Widget};
use serde::{Deserialize, Serialize};

use super::{
    Ctx, Defaults, Family, Part, arg_bool, arg_f64, arg_str, choice, fmt, group, index_of, length,
    note, number, text, toggle,
};
use crate::diagram::{Sketch, shank};
use crate::geom::{self, revolve};
use crate::standards;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RodKind {
    Shaft,
    Threaded,
    LeadScrew,
    Dowel,
}

impl RodKind {
    pub const ALL: [RodKind; 4] = [
        RodKind::Shaft,
        RodKind::Threaded,
        RodKind::LeadScrew,
        RodKind::Dowel,
    ];

    pub fn name(self) -> &'static str {
        match self {
            RodKind::Shaft => "Smooth shaft",
            RodKind::Threaded => "Threaded rod",
            RodKind::LeadScrew => "Lead screw",
            RodKind::Dowel => "Dowel pin",
        }
    }

    fn short(self) -> &'static str {
        match self {
            RodKind::Shaft => "shaft",
            RodKind::Threaded => "threaded rod",
            RodKind::LeadScrew => "lead screw",
            RodKind::Dowel => "dowel pin",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            RodKind::Shaft => "rod",
            RodKind::Threaded => "rod-threaded",
            RodKind::LeadScrew => "rod-lead-screw",
            RodKind::Dowel => "rod-dowel",
        }
    }

    pub fn tool(self) -> &'static str {
        match self {
            RodKind::Shaft => "shaft",
            RodKind::Threaded => "threaded_rod",
            RodKind::LeadScrew => "lead_screw",
            RodKind::Dowel => "dowel_pin",
        }
    }

    pub fn named(name: &str) -> Option<RodKind> {
        RodKind::ALL.into_iter().find(|k| {
            k.tool().eq_ignore_ascii_case(name)
                || k.name().eq_ignore_ascii_case(name)
                || k.short().eq_ignore_ascii_case(name)
                || (name.eq_ignore_ascii_case("rod") && *k == RodKind::Shaft)
                || (name.eq_ignore_ascii_case("dowel") && *k == RodKind::Dowel)
        })
    }

    /// The sizes offered, as the panel names them.
    pub fn sizes(self) -> Vec<String> {
        match self {
            RodKind::Shaft => standards::SHAFT_DIAMETERS
                .iter()
                .map(|d| format!("Ø{}", fmt(*d)))
                .collect(),
            RodKind::Threaded => standards::METRIC_SIZES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            RodKind::LeadScrew => standards::LEAD_SCREWS
                .iter()
                .map(|(n, ..)| n.to_string())
                .collect(),
            RodKind::Dowel => standards::DOWEL_DIAMETERS
                .iter()
                .map(|d| format!("Ø{}", fmt(*d)))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rod {
    pub kind: RodKind,
    pub size: String,
    pub d: f64,
    /// A thread's pitch; 0 on a plain rod.
    #[serde(default)]
    pub pitch: f64,
    /// A lead screw's lead.
    #[serde(default)]
    pub lead: f64,
    pub length: f64,
    #[serde(default)]
    pub custom: bool,
    #[serde(default)]
    pub thread: bool,
    /// The angle about the axis the thread starts at; see
    /// `geom::THREAD_PHASE_DEG` for why it is offered.
    #[serde(default = "phase")]
    pub thread_phase: f64,
}

fn phase() -> f64 {
    geom::THREAD_PHASE_DEG
}

impl Rod {
    pub fn new(kind: RodKind, size: &str, defaults: &Defaults) -> Rod {
        let mut rod = Rod {
            kind,
            size: size.into(),
            d: 0.0,
            pitch: 0.0,
            lead: 0.0,
            length: 100.0,
            custom: false,
            thread: defaults.thread && kind == RodKind::Threaded,
            thread_phase: geom::THREAD_PHASE_DEG,
        };
        rod.refill();
        if kind == RodKind::Dowel {
            rod.length = (4.0 * rod.d).max(8.0);
        }
        rod
    }

    /// The diameter a size names, for sizes written `Ø8`, `8` or `M8`.
    fn diameter_named(size: &str) -> Option<f64> {
        let digits = size.trim().trim_start_matches(['Ø', 'M', 'm', 'ø']);
        digits.parse().ok()
    }

    pub fn refill(&mut self) {
        let sizes = self.kind.sizes();
        let index = sizes
            .iter()
            .position(|s| s.eq_ignore_ascii_case(&self.size))
            .or_else(|| {
                let want = Self::diameter_named(&self.size)?;
                let ds: Vec<f64> = match self.kind {
                    RodKind::Shaft => standards::SHAFT_DIAMETERS.to_vec(),
                    RodKind::Threaded => standards::METRIC_SIZES
                        .iter()
                        .filter_map(|s| standards::metric(s))
                        .map(|(d, _)| d)
                        .collect(),
                    RodKind::LeadScrew => {
                        standards::LEAD_SCREWS.iter().map(|(_, d, ..)| *d).collect()
                    }
                    RodKind::Dowel => standards::DOWEL_DIAMETERS.to_vec(),
                };
                ds.iter().position(|d| (*d - want).abs() < 1e-9)
            })
            .unwrap_or(match self.kind {
                RodKind::Shaft | RodKind::Dowel => 4,
                _ => 0,
            })
            .min(sizes.len() - 1);
        self.size = sizes[index].clone();
        match self.kind {
            RodKind::Shaft => {
                self.d = standards::SHAFT_DIAMETERS[index];
                self.pitch = 0.0;
                self.lead = 0.0;
            }
            RodKind::Dowel => {
                self.d = standards::DOWEL_DIAMETERS[index];
                self.pitch = 0.0;
                self.lead = 0.0;
            }
            RodKind::Threaded => {
                let (d, p) =
                    standards::metric(standards::METRIC_SIZES[index]).unwrap_or((8.0, 1.25));
                self.d = d;
                self.pitch = p;
                self.lead = p;
            }
            RodKind::LeadScrew => {
                let (_, d, p, lead) = standards::LEAD_SCREWS[index];
                self.d = d;
                self.pitch = p;
                self.lead = lead;
            }
        }
    }
}

impl Part for Rod {
    const FAMILY: Family = Family::Rod;

    fn label(&self) -> String {
        match self.kind {
            RodKind::Threaded => {
                format!("{} × {} {}", self.size, fmt(self.length), self.kind.short())
            }
            RodKind::LeadScrew => format!("{} × {}", self.size, fmt(self.length)),
            _ => format!(
                "Ø{} × {} {}",
                fmt(self.d),
                fmt(self.length),
                self.kind.short()
            ),
        }
    }

    fn icon(&self) -> &'static str {
        self.kind.icon()
    }

    fn problem(&self) -> Option<String> {
        if self.d <= 0.0 || self.length <= 0.0 {
            return Some("The diameter and the length must be more than 0.".into());
        }
        if self.thread && (self.pitch <= 0.0 || self.pitch * 1.3 >= self.d) {
            return Some("The thread needs a pitch finer than the diameter allows.".into());
        }
        None
    }

    fn axis(&self) -> ([f64; 3], [f64; 3]) {
        super::z_axis(0.0, self.length)
    }

    fn ops(&self) -> Vec<SolidOp> {
        let (r, l) = (self.d / 2.0, self.length);
        let c = match self.kind {
            RodKind::Dowel => (0.15 * self.d).min(l / 4.0),
            RodKind::Threaded | RodKind::LeadScrew => (0.6 * self.pitch).min(r / 2.0).min(l / 4.0),
            RodKind::Shaft => (0.5f64).min(r / 4.0).min(l / 4.0),
        };
        let mut ops = vec![revolve(
            &[
                [0.0, 0.0],
                [r - c, 0.0],
                [r, c],
                [r, l - c],
                [r - c, l],
                [0.0, l],
            ],
            BooleanOp::NewSolid,
        )];
        if self.thread && self.kind == RodKind::Threaded {
            ops.push(geom::external_thread(
                self.d,
                self.pitch,
                l,
                l,
                false,
                self.thread_phase,
            ));
        }
        if self.thread && self.kind == RodKind::LeadScrew {
            // A trapezoidal thread, half a pitch deep, one start at its lead.
            ops.push(geom::thread_groove(
                r,
                r - 0.5 * self.pitch,
                self.lead,
                l,
                l,
                false,
                false,
                self.thread_phase,
                false,
            ));
        }
        ops
    }

    fn panel(&self, ctx: &Ctx) -> Vec<Widget> {
        let kinds: Vec<&str> = RodKind::ALL.iter().map(|k| k.name()).collect();
        let sizes = self.kind.sizes();
        let mut widgets = vec![
            self.drawing(ctx),
            choice(
                "kind",
                "Rod",
                &kinds,
                RodKind::ALL
                    .iter()
                    .position(|k| *k == self.kind)
                    .unwrap_or(0),
            ),
            choice("size", "Size", &sizes, index_of(&sizes, &self.size)),
            number(ctx, "length", "Length", self.length, 0.1, 1),
        ];
        let mut dims = vec![toggle("custom", "Custom dimensions", self.custom)];
        if self.custom {
            dims.push(number(ctx, "d", "Diameter", self.d, 0.1, 2));
            if matches!(self.kind, RodKind::Threaded | RodKind::LeadScrew) {
                dims.push(number(ctx, "pitch", "Pitch", self.pitch, 0.05, 2));
            }
            if self.kind == RodKind::LeadScrew {
                dims.push(number(ctx, "lead", "Lead", self.lead, 0.05, 2));
            }
        }
        widgets.push(group("Dimensions", self.custom, dims));
        if matches!(self.kind, RodKind::Threaded | RodKind::LeadScrew) {
            let mut options = vec![toggle("thread", "Modelled thread", self.thread)];
            if self.thread {
                options.push(super::angle(
                    ctx,
                    "thread_phase",
                    "Thread start angle",
                    self.thread_phase,
                ));
                options.push(text(
                    "A thread the length of a rod takes the kernel a while. Should the kernel refuse it, change the start angle.",
                ));
            }
            if self.kind == RodKind::LeadScrew && self.lead > self.pitch * 1.5 {
                options.push(text(
                    "A multi-start lead screw is modelled with one start at its lead.",
                ));
            }
            widgets.push(group("Options", true, options));
        }
        widgets.extend(note(self.problem()));
        widgets
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            length("d", "Diameter"),
            length("pitch", "Pitch"),
            length("lead", "Lead"),
            length("length", "Length"),
            super::angle_param("thread_phase", "Thread start angle"),
        ]
    }

    fn apply(&mut self, event: &PanelEvent) -> bool {
        match event {
            PanelEvent::Choice { id, index } => match id.as_str() {
                "kind" => {
                    self.kind = RodKind::ALL.get(*index).copied().unwrap_or(RodKind::Shaft);
                    self.size = format!("Ø{}", fmt(self.d));
                    self.refill();
                    if !matches!(self.kind, RodKind::Threaded | RodKind::LeadScrew) {
                        self.thread = false;
                    }
                }
                "size" => {
                    if let Some(size) = self.kind.sizes().get(*index) {
                        self.size = size.clone();
                        self.refill();
                    }
                }
                _ => return false,
            },
            PanelEvent::Number { id, value } => match id.as_str() {
                "d" => self.d = *value,
                "pitch" => self.pitch = *value,
                "lead" => self.lead = *value,
                "length" => self.length = *value,
                "thread_phase" => self.thread_phase = *value,
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
            None => RodKind::Shaft,
            Some(name) => RodKind::named(name).ok_or_else(|| {
                format!(
                    "`{name}` is not a rod: {}",
                    RodKind::ALL.map(|k| k.tool()).join(", ")
                )
            })?,
        };
        let size = arg_str(args, "size")
            .map(str::to_string)
            .or_else(|| arg_f64(args, "d").map(|d| format!("Ø{}", fmt(d))));
        let mut rod = Rod::new(kind, size.as_deref().unwrap_or("Ø8"), defaults);
        if let Some(size) = &size
            && !rod.size.eq_ignore_ascii_case(size)
            && Self::diameter_named(size) != Some(rod.d)
        {
            if let Some(d) = Self::diameter_named(size) {
                rod.d = d;
                rod.custom = true;
            } else {
                return Err(format!("`{size}` is not a {} size", kind.short()));
            }
        }
        if let Some(v) = arg_f64(args, "length") {
            rod.length = v;
        }
        for (key, slot) in [("pitch", &mut rod.pitch), ("lead", &mut rod.lead)] {
            if let Some(v) = arg_f64(args, key) {
                *slot = v;
                rod.custom = true;
            }
        }
        if let Some(on) = arg_bool(args, "thread") {
            rod.thread = on;
        }
        if let Some(v) = arg_f64(args, "thread_phase") {
            rod.thread_phase = v;
        }
        Ok(rod)
    }
}

impl Rod {
    fn drawing(&self, ctx: &Ctx) -> Widget {
        let mut s = Sketch::new(ctx.focus);
        let r = self.d / 2.0;
        let (outline, drawn, broken) = shank(0.0, r, self.length, self.length);
        let off = Sketch::standoff(drawn.max(self.d));
        s.poly(&outline, DiagramStroke::Outline, !broken);
        if matches!(self.kind, RodKind::Threaded | RodKind::LeadScrew) {
            let n = 7;
            for i in 1..n {
                let y = self.length - drawn * i as f64 / n as f64;
                s.line(
                    &[[-r, y], [r, y - drawn / n as f64 * 0.4]],
                    DiagramStroke::Thin,
                );
            }
        }
        s.axis(
            0.0,
            self.length - drawn - off * 0.4,
            self.length + off * 0.4,
        );
        s.height(
            "length",
            r,
            self.length - drawn,
            self.length,
            -off,
            format!("L {}", fmt(self.length)),
        );
        let d_text = match self.kind {
            RodKind::Threaded => format!("{} × {}", self.size, fmt(self.pitch)),
            RodKind::LeadScrew => format!(
                "Ø{} p{} lead {}",
                fmt(self.d),
                fmt(self.pitch),
                fmt(self.lead)
            ),
            _ => format!("Ø{}", fmt(self.d)),
        };
        s.width("d", -r, r, self.length - drawn, -off, d_text);
        s.finish("rod")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printcad_bench_sdk::json;

    #[test]
    fn a_shaft_is_8_by_100_unless_asked_otherwise() {
        let rod = Rod::new(RodKind::Shaft, "Ø8", &Defaults::default());
        assert_eq!((rod.d, rod.length), (8.0, 100.0));
        assert_eq!(rod.label(), "Ø8 × 100 shaft");
        assert_eq!(rod.ops().len(), 1);
    }

    #[test]
    fn a_threaded_rod_takes_its_pitch_from_the_size() {
        let mut rod = Rod::new(RodKind::Threaded, "M8", &Defaults::default());
        assert_eq!((rod.d, rod.pitch), (8.0, 1.25));
        rod.thread = true;
        assert_eq!(rod.ops().len(), 2);
        assert!(rod.apply(&PanelEvent::Choice {
            id: "kind".into(),
            index: 2,
        }));
        assert_eq!(
            (rod.kind, rod.size.as_str(), rod.lead),
            (RodKind::LeadScrew, "T8 (lead 8)", 8.0)
        );
    }

    #[test]
    fn a_dowel_is_short_and_a_command_names_its_rod() {
        let dowel = Rod::new(RodKind::Dowel, "Ø3", &Defaults::default());
        assert_eq!((dowel.d, dowel.length), (3.0, 12.0));
        let rod = Rod::with_args(
            &json!({"kind": "threaded_rod", "size": "M5", "length": 50}),
            &Defaults::default(),
        )
        .unwrap();
        assert_eq!((rod.d, rod.length), (5.0, 50.0));
        let odd = Rod::with_args(&json!({"kind": "shaft", "d": 7}), &Defaults::default()).unwrap();
        assert!(odd.custom && odd.d == 7.0);
        assert!(Rod::with_args(&json!({"kind": "tube"}), &Defaults::default()).is_err());
    }
}
