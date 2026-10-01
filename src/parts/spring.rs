//! Compression springs: round wire wound about Z from `z = 0`.

use printcad_bench_sdk::Value;
use printcad_bench_sdk::api::kernel_api::{BooleanOp, Profile, ProfileWire, SolidOp, SweepKind};
use printcad_bench_sdk::api::{DiagramStroke, PanelEvent, Parameter, Widget};
use serde::{Deserialize, Serialize};

use super::{
    Ctx, Defaults, Family, Part, arg_f64, count, fmt, integer, length, note, number, text,
};
use crate::diagram::Sketch;
use crate::geom;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spring {
    /// The wire's diameter.
    pub wire: f64,
    /// The outside diameter of the coils.
    pub outer: f64,
    /// The free length.
    pub length: f64,
    pub turns: f64,
}

impl Default for Spring {
    fn default() -> Self {
        Self {
            wire: 1.0,
            outer: 10.0,
            length: 25.0,
            turns: 8.0,
        }
    }
}

impl Spring {
    /// The coil's mean radius.
    fn mean_radius(&self) -> f64 {
        (self.outer - self.wire) / 2.0
    }

    /// The coil's rise per turn.
    pub fn pitch(&self) -> f64 {
        (self.length - self.wire) / self.turns
    }
}

impl Part for Spring {
    const FAMILY: Family = Family::Spring;

    fn label(&self) -> String {
        format!("Ø{} × {} spring", fmt(self.outer), fmt(self.length))
    }

    fn icon(&self) -> &'static str {
        "spring"
    }

    fn problem(&self) -> Option<String> {
        if self.wire <= 0.0 || self.outer <= 0.0 || self.length <= 0.0 || self.turns < 1.0 {
            return Some(
                "The wire, the diameter, the length and at least one turn are needed.".into(),
            );
        }
        if self.outer <= 2.0 * self.wire {
            return Some("The coils must be wider than two wires.".into());
        }
        if self.pitch() <= self.wire * 1.02 {
            return Some("The coils touch: fewer turns, a longer spring or a thinner wire.".into());
        }
        if self.turns > 60.0 {
            return Some("At most 60 turns.".into());
        }
        None
    }

    fn axis(&self) -> ([f64; 3], [f64; 3]) {
        super::z_axis(0.0, self.length)
    }

    fn ops(&self) -> Vec<SolidOp> {
        // The wire's section sits in a plane through the axis, out at the
        // mean radius and a wire's radius up, and is carried round a helix
        // about Z: swept square to its path, as a thread's profile is. (In
        // the plane square to the axis it would stay level as it climbed
        // and sweep a ribbon, not a wire.)
        let r = self.mean_radius();
        vec![SolidOp::Sweep {
            profile: Profile {
                plane: geom::section(),
                wires: vec![ProfileWire::new(vec![geom::circle_at(
                    [r, self.wire / 2.0],
                    self.wire,
                )])],
            },
            kind: SweepKind::Helix {
                axis_origin: [0.0, 0.0],
                axis_dir: [0.0, 1.0],
                pitch: self.pitch(),
                height: self.length - self.wire,
                left_handed: false,
                cone_angle_deg: 0.0,
                reversed: false,
                turns: Some(self.turns),
                growth: None,
            },
            op: BooleanOp::NewSolid,
        }]
    }

    fn panel(&self, ctx: &Ctx) -> Vec<Widget> {
        let mut widgets = vec![
            self.drawing(ctx),
            number(ctx, "outer", "Outside diameter", self.outer, 0.1, 2),
            number(ctx, "wire", "Wire", self.wire, 0.05, 2),
            number(ctx, "length", "Free length", self.length, 0.1, 1),
            count(ctx, "turns", "Turns", self.turns, 1.0),
            text(format!("Pitch {} mm", fmt(self.pitch()))),
        ];
        widgets.extend(note(self.problem()));
        widgets
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            length("outer", "Outside diameter"),
            length("wire", "Wire"),
            length("length", "Free length"),
            integer("turns", "Turns"),
        ]
    }

    fn apply(&mut self, event: &PanelEvent) -> bool {
        match event {
            PanelEvent::Number { id, value } => match id.as_str() {
                "outer" => self.outer = *value,
                "wire" => self.wire = *value,
                "length" => self.length = *value,
                "turns" => self.turns = value.round().max(1.0),
                _ => return false,
            },
            _ => return false,
        }
        true
    }

    fn with_args(args: &Value, _defaults: &Defaults) -> Result<Self, String> {
        let mut spring = Spring::default();
        for (key, slot) in [
            ("outer", &mut spring.outer),
            ("wire", &mut spring.wire),
            ("length", &mut spring.length),
            ("turns", &mut spring.turns),
        ] {
            if let Some(v) = arg_f64(args, key) {
                *slot = v;
            }
        }
        Ok(spring)
    }
}

impl Spring {
    /// The coils as a zigzag between the outside lines.
    fn drawing(&self, ctx: &Ctx) -> Widget {
        let mut s = Sketch::new(ctx.focus);
        let (r, l, w) = (self.outer / 2.0, self.length, self.wire);
        let off = Sketch::standoff(self.outer.max(l));
        let n = (self.turns.max(1.0).round() as usize).min(60);
        let rise = (l - w) / n as f64;
        let mut front = Vec::with_capacity(2 * n + 1);
        for i in 0..=n {
            let y = w / 2.0 + rise * i as f64;
            front.push([
                if i % 2 == 0 {
                    -(r - w / 2.0)
                } else {
                    r - w / 2.0
                },
                y,
            ]);
            if i < n {
                front.push([
                    if i % 2 == 0 {
                        r - w / 2.0
                    } else {
                        -(r - w / 2.0)
                    },
                    y + rise / 2.0,
                ]);
            }
        }
        s.line(&front, DiagramStroke::Outline);
        s.line(&[[-r, 0.0], [-r, l]], DiagramStroke::Thin);
        s.line(&[[r, 0.0], [r, l]], DiagramStroke::Thin);
        s.axis(0.0, -off * 0.4, l + off * 0.4);
        s.width("outer", -r, r, l, off, format!("Ø{}", fmt(self.outer)));
        s.height("length", r, 0.0, l, -off, format!("L {}", fmt(l)));
        s.callout(
            "wire",
            [r - w / 2.0, w / 2.0 + rise * 1.5],
            [r + off * 1.6, w / 2.0],
            format!("wire {}", fmt(w)),
        );
        s.callout(
            "turns",
            [-(r - w / 2.0), w / 2.0 + rise * 2.0],
            [-r - off * 1.6, l * 0.5],
            format!("{} turns", fmt(self.turns)),
        );
        s.finish("spring")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printcad_bench_sdk::json;

    #[test]
    fn a_spring_climbs_its_turns_over_its_length() {
        let spring = Spring::default();
        assert_eq!(spring.problem(), None);
        assert_eq!(spring.pitch(), 3.0);
        let SolidOp::Sweep {
            kind: SweepKind::Helix { height, turns, .. },
            ..
        } = &spring.ops()[0]
        else {
            panic!("a helix");
        };
        assert_eq!((*height, *turns), (24.0, Some(8.0)));
    }

    #[test]
    fn touching_coils_are_refused() {
        let spring = Spring::with_args(&json!({"turns": 30}), &Defaults::default()).unwrap();
        assert!(spring.problem().is_some());
    }
}
