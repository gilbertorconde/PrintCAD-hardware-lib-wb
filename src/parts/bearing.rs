//! Bearings: deep groove ball bearings and linear ball bushings by their
//! designations, on `z = 0` with their axis up Z.

use printcad_bench_sdk::Value;
use printcad_bench_sdk::api::kernel_api::{BooleanOp, SolidOp};
use printcad_bench_sdk::api::{DiagramStroke, PanelEvent, Parameter, Widget};
use serde::{Deserialize, Serialize};

use super::{
    Ctx, Defaults, Family, Part, arg_f64, arg_str, choice, fmt, group, index_of, length, note,
    number, toggle,
};
use crate::diagram::Sketch;
use crate::geom::revolve;
use crate::standards::{self, BearingRow};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BearingKind {
    Ball,
    Linear,
}

impl BearingKind {
    pub const ALL: [BearingKind; 2] = [BearingKind::Ball, BearingKind::Linear];

    pub fn name(self) -> &'static str {
        match self {
            BearingKind::Ball => "Ball bearing",
            BearingKind::Linear => "Linear bushing",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            BearingKind::Ball => "bearing",
            BearingKind::Linear => "bearing-linear",
        }
    }

    pub fn tool(self) -> &'static str {
        match self {
            BearingKind::Ball => "bearing",
            BearingKind::Linear => "linear_bearing",
        }
    }

    /// The word a command names the kind by.
    pub fn word(self) -> &'static str {
        match self {
            BearingKind::Ball => "ball",
            BearingKind::Linear => "linear",
        }
    }

    pub fn named(name: &str) -> Option<BearingKind> {
        BearingKind::ALL.into_iter().find(|k| {
            k.word().eq_ignore_ascii_case(name)
                || k.tool().eq_ignore_ascii_case(name)
                || k.name().eq_ignore_ascii_case(name)
        })
    }

    pub fn rows(self) -> &'static [BearingRow] {
        match self {
            BearingKind::Ball => &standards::BALL_BEARINGS,
            BearingKind::Linear => &standards::LINEAR_BEARINGS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bearing {
    pub kind: BearingKind,
    pub name: String,
    pub bore: f64,
    pub outer: f64,
    pub width: f64,
    #[serde(default)]
    pub custom: bool,
}

impl Bearing {
    pub fn new(kind: BearingKind, name: &str) -> Bearing {
        let mut bearing = Bearing {
            kind,
            name: name.into(),
            bore: 0.0,
            outer: 0.0,
            width: 0.0,
            custom: false,
        };
        bearing.refill();
        bearing
    }

    fn names(&self) -> Vec<&'static str> {
        self.kind.rows().iter().map(|r| r.name).collect()
    }

    pub fn refill(&mut self) {
        let rows = self.kind.rows();
        let row = rows
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(&self.name))
            .or_else(|| rows.iter().find(|r| r.bore == self.bore))
            .copied()
            .unwrap_or(rows[0]);
        self.name = row.name.into();
        self.bore = row.bore;
        self.outer = row.outer;
        self.width = row.width;
    }
}

impl Part for Bearing {
    const FAMILY: Family = Family::Bearing;

    fn label(&self) -> String {
        match self.kind {
            BearingKind::Ball => format!("{} bearing", self.name),
            BearingKind::Linear => self.name.clone(),
        }
    }

    fn icon(&self) -> &'static str {
        self.kind.icon()
    }

    fn problem(&self) -> Option<String> {
        if self.bore <= 0.0 || self.width <= 0.0 {
            return Some("The bore and the width must be more than 0.".into());
        }
        if self.outer <= self.bore + 1.0 {
            return Some("The rings are thinner than 0.5 mm.".into());
        }
        None
    }

    fn axis(&self) -> ([f64; 3], [f64; 3]) {
        super::z_axis(0.0, self.width)
    }

    fn ops(&self) -> Vec<SolidOp> {
        let (r0, r1, b) = (self.bore / 2.0, self.outer / 2.0, self.width);
        let c = (0.3f64).min(b / 8.0).min((r1 - r0) / 6.0);
        match self.kind {
            BearingKind::Ball => {
                // The two rings, parted by a shallow groove on each face.
                let rm = (r0 + r1) / 2.0;
                let (gw, gd) = (((r1 - r0) * 0.12).max(0.2), (0.3f64).min(b / 6.0));
                vec![revolve(
                    &[
                        [r0, c],
                        [r0 + c, 0.0],
                        [rm - gw, 0.0],
                        [rm - gw, gd],
                        [rm + gw, gd],
                        [rm + gw, 0.0],
                        [r1 - c, 0.0],
                        [r1, c],
                        [r1, b - c],
                        [r1 - c, b],
                        [rm + gw, b],
                        [rm + gw, b - gd],
                        [rm - gw, b - gd],
                        [rm - gw, b],
                        [r0 + c, b],
                        [r0, b - c],
                    ],
                    BooleanOp::NewSolid,
                )]
            }
            BearingKind::Linear => {
                // A sleeve with a retaining-ring groove near each end.
                let (gw, gd) = (1.1f64.min(b / 10.0), 0.5f64.min((r1 - r0) / 4.0));
                let g0 = (0.12 * b).max(gw);
                vec![revolve(
                    &[
                        [r0, c],
                        [r0 + c, 0.0],
                        [r1 - c, 0.0],
                        [r1, c],
                        [r1, g0],
                        [r1 - gd, g0],
                        [r1 - gd, g0 + gw],
                        [r1, g0 + gw],
                        [r1, b - g0 - gw],
                        [r1 - gd, b - g0 - gw],
                        [r1 - gd, b - g0],
                        [r1, b - g0],
                        [r1, b - c],
                        [r1 - c, b],
                        [r0 + c, b],
                        [r0, b - c],
                    ],
                    BooleanOp::NewSolid,
                )]
            }
        }
    }

    fn panel(&self, ctx: &Ctx) -> Vec<Widget> {
        let kinds: Vec<&str> = BearingKind::ALL.iter().map(|k| k.name()).collect();
        let names = self.names();
        let mut widgets = vec![
            self.drawing(ctx),
            choice(
                "kind",
                "Bearing",
                &kinds,
                BearingKind::ALL
                    .iter()
                    .position(|k| *k == self.kind)
                    .unwrap_or(0),
            ),
            choice("name", "Designation", &names, index_of(&names, &self.name)),
        ];
        let mut dims = vec![toggle("custom", "Custom dimensions", self.custom)];
        if self.custom {
            dims.push(number(ctx, "bore", "Bore", self.bore, 0.1, 2));
            dims.push(number(ctx, "outer", "Outside", self.outer, 0.1, 2));
            dims.push(number(ctx, "width", "Width", self.width, 0.1, 2));
        }
        widgets.push(group("Dimensions", self.custom, dims));
        widgets.extend(note(self.problem()));
        widgets
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            length("bore", "Bore"),
            length("outer", "Outside"),
            length("width", "Width"),
        ]
    }

    fn apply(&mut self, event: &PanelEvent) -> bool {
        match event {
            PanelEvent::Choice { id, index } => match id.as_str() {
                "kind" => {
                    self.kind = BearingKind::ALL
                        .get(*index)
                        .copied()
                        .unwrap_or(BearingKind::Ball);
                    self.name.clear();
                    self.refill();
                }
                "name" => {
                    if let Some(name) = self.names().get(*index) {
                        self.name = (*name).into();
                        self.refill();
                    }
                }
                _ => return false,
            },
            PanelEvent::Number { id, value } => match id.as_str() {
                "bore" => self.bore = *value,
                "outer" => self.outer = *value,
                "width" => self.width = *value,
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

    fn with_args(args: &Value, _defaults: &Defaults) -> Result<Self, String> {
        let name = arg_str(args, "name").or(arg_str(args, "designation"));
        let kind = match arg_str(args, "kind") {
            Some(k) => BearingKind::named(k)
                .ok_or_else(|| format!("`{k}` is not a bearing: ball or linear"))?,
            None if name.is_some_and(|n| n.to_uppercase().starts_with("LM")) => BearingKind::Linear,
            None => BearingKind::Ball,
        };
        let name = name.unwrap_or(if kind == BearingKind::Ball {
            "608"
        } else {
            "LM8UU"
        });
        let mut bearing = Bearing::new(kind, name);
        if !bearing.name.eq_ignore_ascii_case(name) {
            return Err(format!(
                "no {} is listed as `{name}`",
                kind.name().to_lowercase()
            ));
        }
        for (key, slot) in [
            ("bore", &mut bearing.bore),
            ("outer", &mut bearing.outer),
            ("width", &mut bearing.width),
        ] {
            if let Some(v) = arg_f64(args, key) {
                *slot = v;
                bearing.custom = true;
            }
        }
        Ok(bearing)
    }
}

impl Bearing {
    fn drawing(&self, ctx: &Ctx) -> Widget {
        let mut s = Sketch::new(ctx.focus);
        let (r0, r1, b) = (self.bore / 2.0, self.outer / 2.0, self.width);
        let off = Sketch::standoff(self.outer.max(b));
        s.rect([-r1, 0.0], [r1, b]);
        s.hidden(&[[-r0, 0.0], [-r0, b]]);
        s.hidden(&[[r0, 0.0], [r0, b]]);
        match self.kind {
            BearingKind::Ball => {
                let rm = (r0 + r1) / 2.0;
                for x in [-rm, rm] {
                    s.line(&[[x, 0.0], [x, b]], DiagramStroke::Thin);
                }
            }
            BearingKind::Linear => {
                for y in [0.12 * b, 0.88 * b] {
                    s.line(&[[-r1, y], [r1, y]], DiagramStroke::Thin);
                }
            }
        }
        s.axis(0.0, -off * 0.4, b + off * 0.4);
        s.width("outer", -r1, r1, b, off, format!("D {}", fmt(self.outer)));
        s.width("bore", -r0, r0, 0.0, -off, format!("d {}", fmt(self.bore)));
        s.height("width", r1, 0.0, b, -off, format!("B {}", fmt(b)));
        s.finish("bearing")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printcad_bench_sdk::json;

    #[test]
    fn a_608_is_8_by_22_by_7() {
        let bearing = Bearing::new(BearingKind::Ball, "608");
        assert_eq!(
            (bearing.bore, bearing.outer, bearing.width),
            (8.0, 22.0, 7.0)
        );
        assert_eq!(bearing.label(), "608 bearing");
        assert_eq!(bearing.ops().len(), 1);
    }

    #[test]
    fn a_designation_picks_its_kind() {
        let bearing = Bearing::with_args(&json!({"name": "LM8UU"}), &Defaults::default()).unwrap();
        assert_eq!((bearing.kind, bearing.width), (BearingKind::Linear, 24.0));
        assert!(Bearing::with_args(&json!({"name": "6999"}), &Defaults::default()).is_err());
        let mut bearing = Bearing::new(BearingKind::Ball, "625");
        assert!(bearing.apply(&PanelEvent::Choice {
            id: "kind".into(),
            index: 1,
        }));
        assert_eq!(bearing.name, "LM6UU");
    }

    #[test]
    fn a_kind_is_named_by_its_word_tool_or_name() {
        for kind in BearingKind::ALL {
            for name in [kind.word(), kind.tool(), kind.name()] {
                assert_eq!(BearingKind::named(&name.to_uppercase()), Some(kind));
            }
        }
        let defaults = Defaults::default();
        let ball = Bearing::with_args(&json!({"kind": "ball", "name": "608"}), &defaults).unwrap();
        assert_eq!(ball.kind, BearingKind::Ball);
        let linear = Bearing::with_args(&json!({"kind": "linear"}), &defaults).unwrap();
        assert_eq!(linear.kind, BearingKind::Linear);
        assert!(Bearing::with_args(&json!({"kind": "roller"}), &defaults).is_err());
    }
}
