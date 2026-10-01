//! Washers: plain, large and split spring lock washers, on `z = 0` with
//! their axis up Z.

use printcad_bench_sdk::Value;
use printcad_bench_sdk::api::kernel_api::{BooleanOp, Profile, ProfileWire, SolidOp, SweepKind};
use printcad_bench_sdk::api::{DiagramStroke, PanelEvent, Parameter, Widget};
use serde::{Deserialize, Serialize};

use super::{
    Ctx, Defaults, Family, Part, arg_f64, arg_str, choice, fmt, group, index_of, length, note,
    number, toggle,
};
use crate::diagram::Sketch;
use crate::geom;
use crate::standards::{self, WasherTable};

/// How far round a spring washer runs: short of a turn, so its ends
/// stand apart.
const SPRING_TURNS: f64 = 0.96;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasherKind {
    Flat,
    Large,
    Spring,
}

impl WasherKind {
    pub const ALL: [WasherKind; 3] = [WasherKind::Flat, WasherKind::Large, WasherKind::Spring];

    pub fn name(self) -> &'static str {
        match self {
            WasherKind::Flat => "Plain",
            WasherKind::Large => "Large plain",
            WasherKind::Spring => "Spring lock",
        }
    }

    fn short(self) -> &'static str {
        match self {
            WasherKind::Flat => "washer",
            WasherKind::Large => "large washer",
            WasherKind::Spring => "spring washer",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            WasherKind::Flat => "washer-flat",
            WasherKind::Large => "washer-large",
            WasherKind::Spring => "washer-spring",
        }
    }

    pub fn tool(self) -> &'static str {
        match self {
            WasherKind::Flat => "washer",
            WasherKind::Large => "large_washer",
            WasherKind::Spring => "spring_washer",
        }
    }

    pub fn named(name: &str) -> Option<WasherKind> {
        WasherKind::ALL.into_iter().find(|k| {
            k.tool().eq_ignore_ascii_case(name)
                || k.name().eq_ignore_ascii_case(name)
                || k.tool()
                    .trim_end_matches("_washer")
                    .eq_ignore_ascii_case(name)
        })
    }

    pub fn table(self) -> &'static WasherTable {
        match self {
            WasherKind::Flat => &standards::ISO_7089,
            WasherKind::Large => &standards::ISO_7093,
            WasherKind::Spring => &standards::DIN_127,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Washer {
    pub kind: WasherKind,
    pub standard: String,
    pub size: String,
    /// The hole, the outside and the thickness.
    pub d1: f64,
    pub d2: f64,
    pub h: f64,
    #[serde(default)]
    pub custom: bool,
}

impl Washer {
    pub fn new(kind: WasherKind, size: &str) -> Washer {
        let mut washer = Washer {
            kind,
            standard: kind.table().name.into(),
            size: size.into(),
            d1: 0.0,
            d2: 0.0,
            h: 0.0,
            custom: false,
        };
        washer.refill();
        washer
    }

    fn sizes(&self) -> Vec<&'static str> {
        self.kind.table().rows.iter().map(|r| r.size).collect()
    }

    pub fn refill(&mut self) {
        let table = self.kind.table();
        self.standard = table.name.into();
        let row = table
            .rows
            .iter()
            .find(|r| r.size.eq_ignore_ascii_case(&self.size))
            .or_else(|| {
                let want = standards::metric(&self.size)
                    .map(|(d, _)| d)
                    .unwrap_or(self.d1);
                table
                    .rows
                    .iter()
                    .min_by(|a, b| (a.d1 - want).abs().total_cmp(&(b.d1 - want).abs()))
            })
            .copied()
            .unwrap_or(table.rows[0]);
        self.size = row.size.into();
        self.d1 = row.d1;
        self.d2 = row.d2;
        self.h = row.h;
    }
}

impl Part for Washer {
    const FAMILY: Family = Family::Washer;

    fn label(&self) -> String {
        format!("{} {}", self.size, self.kind.short())
    }

    fn icon(&self) -> &'static str {
        self.kind.icon()
    }

    fn problem(&self) -> Option<String> {
        if self.d1 <= 0.0 || self.h <= 0.0 {
            return Some("The hole and the thickness must be more than 0.".into());
        }
        if self.d2 <= self.d1 + 0.4 {
            return Some("The ring is thinner than 0.2 mm.".into());
        }
        None
    }

    fn axis(&self) -> ([f64; 3], [f64; 3]) {
        // A spring washer rises most of a thickness over its turn.
        let top = match self.kind {
            WasherKind::Spring => self.h * (1.0 + SPRING_TURNS),
            _ => self.h,
        };
        super::z_axis(0.0, top)
    }

    fn ops(&self) -> Vec<SolidOp> {
        match self.kind {
            WasherKind::Flat | WasherKind::Large => vec![geom::extrude(
                geom::xy(0.0),
                vec![vec![geom::circle(self.d2)], vec![geom::circle(self.d1)]],
                self.h,
                BooleanOp::NewSolid,
            )],
            WasherKind::Spring => {
                // The ring's section, swept most of a turn along a helix
                // that climbs a thickness, so its ends stand apart.
                let (r0, r1, h) = (self.d1 / 2.0, self.d2 / 2.0, self.h);
                let turns = SPRING_TURNS;
                vec![SolidOp::Sweep {
                    profile: Profile {
                        plane: geom::section(),
                        wires: vec![ProfileWire::new(geom::polygon(&[
                            [r0, 0.0],
                            [r1, 0.0],
                            [r1, h],
                            [r0, h],
                        ]))],
                    },
                    kind: SweepKind::Helix {
                        axis_origin: [0.0, 0.0],
                        axis_dir: [0.0, 1.0],
                        pitch: h,
                        height: h * turns,
                        left_handed: false,
                        cone_angle_deg: 0.0,
                        reversed: false,
                        turns: Some(turns),
                        growth: None,
                    },
                    op: BooleanOp::NewSolid,
                }]
            }
        }
    }

    fn panel(&self, ctx: &Ctx) -> Vec<Widget> {
        let kinds: Vec<&str> = WasherKind::ALL.iter().map(|k| k.name()).collect();
        let sizes = self.sizes();
        let mut widgets = vec![
            self.drawing(ctx),
            choice(
                "kind",
                "Washer",
                &kinds,
                WasherKind::ALL
                    .iter()
                    .position(|k| *k == self.kind)
                    .unwrap_or(0),
            ),
            choice("standard", "Standard", &[self.standard.as_str()], 0),
            choice("size", "Size", &sizes, index_of(&sizes, &self.size)),
        ];
        let mut dims = vec![toggle("custom", "Custom dimensions", self.custom)];
        if self.custom {
            dims.push(number(ctx, "d1", "Hole", self.d1, 0.1, 2));
            dims.push(number(ctx, "d2", "Outside", self.d2, 0.1, 2));
            dims.push(number(ctx, "h", "Thickness", self.h, 0.05, 2));
        }
        widgets.push(group("Dimensions", self.custom, dims));
        widgets.extend(note(self.problem()));
        widgets
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            length("d1", "Hole"),
            length("d2", "Outside"),
            length("h", "Thickness"),
        ]
    }

    fn apply(&mut self, event: &PanelEvent) -> bool {
        match event {
            PanelEvent::Choice { id, index } => match id.as_str() {
                "kind" => {
                    self.kind = WasherKind::ALL
                        .get(*index)
                        .copied()
                        .unwrap_or(WasherKind::Flat);
                    self.refill();
                }
                "size" => {
                    if let Some(size) = self.sizes().get(*index) {
                        self.size = (*size).into();
                        self.refill();
                    }
                }
                "standard" => {}
                _ => return false,
            },
            PanelEvent::Number { id, value } => match id.as_str() {
                "d1" => self.d1 = *value,
                "d2" => self.d2 = *value,
                "h" => self.h = *value,
                _ => return false,
            },
            PanelEvent::Toggle { id, on } if id == "custom" => {
                self.custom = *on;
                if !*on {
                    self.refill();
                }
            }
            _ => return false,
        }
        true
    }

    fn with_args(args: &Value, defaults: &Defaults) -> Result<Self, String> {
        let kind = match arg_str(args, "kind") {
            None => WasherKind::Flat,
            Some(name) => WasherKind::named(name).ok_or_else(|| {
                format!(
                    "`{name}` is not a washer: {}",
                    WasherKind::ALL.map(|k| k.tool()).join(", ")
                )
            })?,
        };
        let size = arg_str(args, "size").unwrap_or(&defaults.size);
        let mut washer = Washer::new(kind, size);
        if !washer.size.eq_ignore_ascii_case(size) && arg_str(args, "size").is_some() {
            return Err(format!("{} has no size `{size}`", washer.standard));
        }
        for (key, slot) in [
            ("d1", &mut washer.d1),
            ("d2", &mut washer.d2),
            ("h", &mut washer.h),
        ] {
            if let Some(v) = arg_f64(args, key) {
                *slot = v;
                washer.custom = true;
            }
        }
        Ok(washer)
    }
}

impl Washer {
    fn drawing(&self, ctx: &Ctx) -> Widget {
        let mut s = Sketch::new(ctx.focus);
        let (r1, r2, h) = (self.d1 / 2.0, self.d2 / 2.0, self.h);
        let off = Sketch::standoff(self.d2);
        // A thickness drawn as it is would vanish beside the width: the
        // view is the face, with the section beside it.
        s.circle([0.0, 0.0], self.d2, DiagramStroke::Outline, true);
        s.circle([0.0, 0.0], self.d1, DiagramStroke::Outline, false);
        if self.kind == WasherKind::Spring {
            let gap = 0.08 * r2;
            s.line(&[[r1, -gap], [r2, -gap]], DiagramStroke::Outline);
            s.line(&[[r1, gap], [r2, gap]], DiagramStroke::Outline);
        }
        let x = r2 + off * 2.2;
        s.rect([x, -r2], [x + h, -r1]);
        s.rect([x, r1], [x + h, r2]);
        s.hidden(&[[x, -r1], [x, r1]]);
        s.hidden(&[[x + h, -r1], [x + h, r1]]);
        s.width("d2", -r2, r2, r2, off, format!("d2 {}", fmt(self.d2)));
        s.width("d1", -r1, r1, -r2, -off, format!("d1 {}", fmt(self.d1)));
        s.width("h", x, x + h, r2, off, format!("h {}", fmt(h)));
        s.finish("washer")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printcad_bench_sdk::json;

    #[test]
    fn a_washer_is_sized_by_its_standard() {
        let washer = Washer::new(WasherKind::Flat, "M4");
        assert_eq!((washer.d1, washer.d2, washer.h), (4.3, 9.0, 0.8));
        assert_eq!(washer.label(), "M4 washer");
        assert_eq!(washer.ops().len(), 1);
    }

    #[test]
    fn a_spring_washer_is_most_of_a_turn_of_a_helix() {
        let washer = Washer::new(WasherKind::Spring, "M5");
        let SolidOp::Sweep {
            kind: SweepKind::Helix { turns, pitch, .. },
            ..
        } = &washer.ops()[0]
        else {
            panic!("a helix");
        };
        assert_eq!(*turns, Some(0.96));
        assert_eq!(*pitch, 1.2);
    }

    #[test]
    fn changing_the_kind_keeps_the_size() {
        let mut washer = Washer::new(WasherKind::Flat, "M6");
        assert!(washer.apply(&PanelEvent::Choice {
            id: "kind".into(),
            index: 1,
        }));
        assert_eq!((washer.size.as_str(), washer.d2), ("M6", 18.0));
        let washer = Washer::with_args(
            &json!({"kind": "spring", "size": "M8"}),
            &Defaults::default(),
        )
        .unwrap();
        assert_eq!((washer.kind, washer.d2), (WasherKind::Spring, 14.8));
    }
}
