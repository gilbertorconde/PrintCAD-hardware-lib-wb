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
    pub outer: f64,
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
        self.length = if self.short { row.short } else { row.length };
        self.hole = row.hole;
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
        if self.outer <= self.d + 0.4 {
            return Some("The wall around the thread is thinner than 0.2 mm.".into());
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
        let (r0, r1, l) = (self.minor() / 2.0, self.outer / 2.0, self.length);
        let taper = (0.3f64).min(l / 6.0).min((r1 - r0) * 0.8);
        // Two knurled bands with a smooth groove between, and a smooth
        // band at the top.
        let g = (0.1f64).min((r1 - r0) * 0.3);
        let (g0, g1) = (0.42 * l, 0.58 * l);
        let top = 0.85 * l;
        let mut ops = vec![revolve(
            &[
                [r0, 0.0],
                [r1 - taper, 0.0],
                [r1, taper],
                [r1, g0],
                [r1 - g, g0],
                [r1 - g, g1],
                [r1, g1],
                [r1, top],
                [r1 - g, top],
                [r1 - g, l],
                [r0, l],
            ],
            BooleanOp::NewSolid,
        )];
        if self.thread {
            ops.push(geom::internal_thread(
                self.d,
                self.minor(),
                self.pitch,
                l,
                l,
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
            choice(
                "short",
                "Length",
                &[
                    format!("Standard ({})", fmt(row.length)),
                    format!("Short ({})", fmt(row.short)),
                ],
                usize::from(self.short),
            ),
            text(format!(
                "Drive it into a Ø{} hole, {} deep or more.",
                fmt(self.hole),
                fmt(self.length + 0.5)
            )),
        ];
        let mut dims = vec![toggle("custom", "Custom dimensions", self.custom)];
        if self.custom {
            dims.push(number(ctx, "d", "Thread diameter", self.d, 0.1, 2));
            dims.push(number(ctx, "pitch", "Pitch", self.pitch, 0.05, 2));
            dims.push(number(ctx, "outer", "Outside", self.outer, 0.1, 2));
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
            length("outer", "Outside"),
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
        let (r0, r1, l) = (self.d / 2.0, self.outer / 2.0, self.length);
        let off = Sketch::standoff(self.outer.max(l));
        s.rect([-r1, 0.0], [r1, l]);
        for y in [0.42 * l, 0.58 * l, 0.85 * l] {
            s.line(&[[-r1, y], [r1, y]], DiagramStroke::Thin);
        }
        s.hidden(&[[-r0, 0.0], [-r0, l]]);
        s.hidden(&[[r0, 0.0], [r0, l]]);
        s.axis(0.0, -off * 0.4, l + off * 0.4);
        s.width("outer", -r1, r1, l, off, format!("Ø{}", fmt(self.outer)));
        s.width(
            "d",
            -r0,
            r0,
            0.0,
            -off,
            format!("{} × {}", self.size, fmt(self.pitch)),
        );
        s.height("length", r1, 0.0, l, -off, format!("L {}", fmt(l)));
        s.callout(
            "hole",
            [-r1, l * 0.3],
            [-r1 - off * 1.8, -off * 0.6],
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
        assert_eq!((insert.outer, insert.length, insert.hole), (4.0, 5.7, 4.0));
        assert_eq!(insert.label(), "M3 insert");
        assert_eq!(insert.ops().len(), 1);
    }

    #[test]
    fn the_short_length_is_a_choice() {
        let mut insert = Insert::new("M4", &Defaults::default());
        assert!(insert.apply(&PanelEvent::Choice {
            id: "short".into(),
            index: 1,
        }));
        assert_eq!(insert.length, 4.7);
        let insert = Insert::with_args(
            &json!({"size": "M5", "short": true, "thread": true}),
            &Defaults::default(),
        )
        .unwrap();
        assert_eq!(insert.length, 5.8);
        assert_eq!(insert.ops().len(), 2);
        assert!(Insert::with_args(&json!({"size": "M8"}), &Defaults::default()).is_err());
    }
}
