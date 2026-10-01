//! Magnets: discs, rings and blocks, on `z = 0` about the origin.

use printcad_bench_sdk::Value;
use printcad_bench_sdk::api::kernel_api::{BooleanOp, SolidOp};
use printcad_bench_sdk::api::{DiagramStroke, PanelEvent, Parameter, Widget};
use serde::{Deserialize, Serialize};

use super::{Ctx, Defaults, Family, Part, arg_f64, arg_str, choice, fmt, length, note, number};
use crate::diagram::Sketch;
use crate::geom;
use crate::standards;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagnetShape {
    Disc,
    Ring,
    Block,
}

impl MagnetShape {
    pub const ALL: [MagnetShape; 3] = [MagnetShape::Disc, MagnetShape::Ring, MagnetShape::Block];

    pub fn name(self) -> &'static str {
        match self {
            MagnetShape::Disc => "Disc",
            MagnetShape::Ring => "Ring",
            MagnetShape::Block => "Block",
        }
    }

    pub fn named(name: &str) -> Option<MagnetShape> {
        MagnetShape::ALL.into_iter().find(|s| {
            s.name().eq_ignore_ascii_case(name)
                || (name.eq_ignore_ascii_case("bar") && *s == MagnetShape::Block)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Magnet {
    pub shape: MagnetShape,
    /// A disc's or ring's diameter.
    pub d: f64,
    /// A ring's hole.
    #[serde(default)]
    pub bore: f64,
    /// A block's length and width.
    pub l: f64,
    pub w: f64,
    pub h: f64,
}

impl Magnet {
    pub fn new(shape: MagnetShape) -> Magnet {
        let mut m = Magnet {
            shape,
            d: 6.0,
            bore: 0.0,
            l: 10.0,
            w: 5.0,
            h: 3.0,
        };
        if shape == MagnetShape::Ring {
            m.d = 10.0;
            m.bore = 4.0;
        }
        if shape == MagnetShape::Block {
            m.h = 2.0;
        }
        m
    }

    fn presets(&self) -> Vec<String> {
        let mut out: Vec<String> = match self.shape {
            MagnetShape::Block => standards::BLOCK_MAGNETS
                .iter()
                .map(|(l, w, h)| format!("{} × {} × {}", fmt(*l), fmt(*w), fmt(*h)))
                .collect(),
            _ => standards::DISC_MAGNETS
                .iter()
                .map(|(d, h)| format!("Ø{} × {}", fmt(*d), fmt(*h)))
                .collect(),
        };
        out.push("Custom".into());
        out
    }

    fn preset_index(&self) -> usize {
        let presets = self.presets();
        match self.shape {
            MagnetShape::Block => standards::BLOCK_MAGNETS
                .iter()
                .position(|(l, w, h)| *l == self.l && *w == self.w && *h == self.h),
            _ => standards::DISC_MAGNETS
                .iter()
                .position(|(d, h)| *d == self.d && *h == self.h),
        }
        .unwrap_or(presets.len() - 1)
    }
}

impl Part for Magnet {
    const FAMILY: Family = Family::Magnet;

    fn label(&self) -> String {
        match self.shape {
            MagnetShape::Disc => format!("Ø{} × {} magnet", fmt(self.d), fmt(self.h)),
            MagnetShape::Ring => format!(
                "Ø{}/{} × {} ring magnet",
                fmt(self.d),
                fmt(self.bore),
                fmt(self.h)
            ),
            MagnetShape::Block => {
                format!("{} × {} × {} magnet", fmt(self.l), fmt(self.w), fmt(self.h))
            }
        }
    }

    fn icon(&self) -> &'static str {
        "magnet"
    }

    fn problem(&self) -> Option<String> {
        if self.h <= 0.0 {
            return Some("The height must be more than 0.".into());
        }
        match self.shape {
            MagnetShape::Disc if self.d <= 0.0 => Some("The diameter must be more than 0.".into()),
            MagnetShape::Ring if self.bore <= 0.0 || self.bore >= self.d - 0.4 => {
                Some("The hole must leave a ring at least 0.2 mm thick.".into())
            }
            MagnetShape::Block if self.l <= 0.0 || self.w <= 0.0 => {
                Some("The sides must be more than 0.".into())
            }
            _ => None,
        }
    }

    fn axis(&self) -> ([f64; 3], [f64; 3]) {
        super::z_axis(0.0, self.h)
    }

    fn ops(&self) -> Vec<SolidOp> {
        let wires = match self.shape {
            MagnetShape::Disc => vec![vec![geom::circle(self.d)]],
            MagnetShape::Ring => vec![vec![geom::circle(self.d)], vec![geom::circle(self.bore)]],
            MagnetShape::Block => vec![geom::polygon(&geom::rectangle(self.l, self.w))],
        };
        vec![geom::extrude(
            geom::xy(0.0),
            wires,
            self.h,
            BooleanOp::NewSolid,
        )]
    }

    fn panel(&self, ctx: &Ctx) -> Vec<Widget> {
        let shapes: Vec<&str> = MagnetShape::ALL.iter().map(|s| s.name()).collect();
        let presets = self.presets();
        let mut widgets = vec![
            self.drawing(ctx),
            choice(
                "shape",
                "Shape",
                &shapes,
                MagnetShape::ALL
                    .iter()
                    .position(|s| *s == self.shape)
                    .unwrap_or(0),
            ),
            choice("preset", "Size", &presets, self.preset_index()),
        ];
        match self.shape {
            MagnetShape::Disc => widgets.push(number(ctx, "d", "Diameter", self.d, 0.1, 2)),
            MagnetShape::Ring => {
                widgets.push(number(ctx, "d", "Diameter", self.d, 0.1, 2));
                widgets.push(number(ctx, "bore", "Hole", self.bore, 0.1, 2));
            }
            MagnetShape::Block => {
                widgets.push(number(ctx, "l", "Length", self.l, 0.1, 2));
                widgets.push(number(ctx, "w", "Width", self.w, 0.1, 2));
            }
        }
        widgets.push(number(ctx, "h", "Height", self.h, 0.1, 2));
        widgets.extend(note(self.problem()));
        widgets
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            length("d", "Diameter"),
            length("bore", "Hole"),
            length("l", "Length"),
            length("w", "Width"),
            length("h", "Height"),
        ]
    }

    fn apply(&mut self, event: &PanelEvent) -> bool {
        match event {
            PanelEvent::Choice { id, index } => match id.as_str() {
                "shape" => {
                    let shape = MagnetShape::ALL
                        .get(*index)
                        .copied()
                        .unwrap_or(MagnetShape::Disc);
                    let keep = self.h;
                    *self = Magnet::new(shape);
                    self.h = keep;
                }
                "preset" => match self.shape {
                    MagnetShape::Block => {
                        if let Some((l, w, h)) = standards::BLOCK_MAGNETS.get(*index) {
                            (self.l, self.w, self.h) = (*l, *w, *h);
                        }
                    }
                    _ => {
                        if let Some((d, h)) = standards::DISC_MAGNETS.get(*index) {
                            (self.d, self.h) = (*d, *h);
                            if self.shape == MagnetShape::Ring && self.bore >= self.d - 0.4 {
                                self.bore = (self.d * 0.4 * 2.0).round() / 2.0;
                            }
                        }
                    }
                },
                _ => return false,
            },
            PanelEvent::Number { id, value } => match id.as_str() {
                "d" => self.d = *value,
                "bore" => self.bore = *value,
                "l" => self.l = *value,
                "w" => self.w = *value,
                "h" => self.h = *value,
                _ => return false,
            },
            _ => return false,
        }
        true
    }

    fn with_args(args: &Value, _defaults: &Defaults) -> Result<Self, String> {
        let shape = match arg_str(args, "shape") {
            None => MagnetShape::Disc,
            Some(name) => MagnetShape::named(name)
                .ok_or_else(|| format!("`{name}` is not a shape: disc, ring or block"))?,
        };
        let mut magnet = Magnet::new(shape);
        for (key, slot) in [
            ("d", &mut magnet.d),
            ("bore", &mut magnet.bore),
            ("l", &mut magnet.l),
            ("w", &mut magnet.w),
            ("h", &mut magnet.h),
        ] {
            if let Some(v) = arg_f64(args, key) {
                *slot = v;
            }
        }
        Ok(magnet)
    }
}

impl Magnet {
    fn drawing(&self, ctx: &Ctx) -> Widget {
        let mut s = Sketch::new(ctx.focus);
        let h = self.h;
        match self.shape {
            MagnetShape::Block => {
                let (l, w) = (self.l / 2.0, self.w / 2.0);
                let off = Sketch::standoff(self.l.max(self.w).max(h));
                // The top, and the front under it.
                let gap = off * 0.8;
                s.rect([-l, gap], [l, gap + self.w]);
                s.rect([-l, 0.0], [l, -h]);
                s.width("l", -l, l, gap + self.w, off, format!("l {}", fmt(self.l)));
                s.height(
                    "w",
                    -l,
                    gap,
                    gap + self.w,
                    off,
                    format!("w {}", fmt(self.w)),
                );
                s.height("h", l, -h, 0.0, -off, format!("h {}", fmt(h)));
                let _ = w;
            }
            _ => {
                let r = self.d / 2.0;
                let off = Sketch::standoff(self.d.max(h));
                let gap = off * 0.8;
                s.circle([0.0, gap + r], self.d, DiagramStroke::Outline, true);
                if self.shape == MagnetShape::Ring {
                    s.circle([0.0, gap + r], self.bore, DiagramStroke::Outline, false);
                    let b = self.bore / 2.0;
                    s.hidden(&[[-b, 0.0], [-b, -h]]);
                    s.hidden(&[[b, 0.0], [b, -h]]);
                    s.width("bore", -b, b, -h, -off, format!("Ø{}", fmt(self.bore)));
                }
                s.rect([-r, 0.0], [r, -h]);
                s.width("d", -r, r, gap + self.d, off, format!("Ø{}", fmt(self.d)));
                s.height("h", r, -h, 0.0, -off, format!("h {}", fmt(h)));
            }
        }
        s.finish("magnet")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printcad_bench_sdk::json;

    #[test]
    fn a_disc_takes_a_preset() {
        let mut m = Magnet::new(MagnetShape::Disc);
        assert_eq!(m.label(), "Ø6 × 3 magnet");
        assert!(m.apply(&PanelEvent::Choice {
            id: "preset".into(),
            index: 9,
        }));
        assert_eq!((m.d, m.h), (10.0, 3.0));
        assert_eq!(m.preset_index(), 9);
        m.d = 7.0;
        assert_eq!(m.preset_index(), standards::DISC_MAGNETS.len(), "custom");
    }

    #[test]
    fn a_ring_keeps_a_wall_and_a_block_has_two_sides() {
        let mut ring = Magnet::new(MagnetShape::Ring);
        assert_eq!(ring.problem(), None);
        ring.bore = 9.8;
        assert!(ring.problem().is_some());
        let block = Magnet::with_args(
            &json!({"shape": "block", "l": 20, "w": 10, "h": 2}),
            &Defaults::default(),
        )
        .unwrap();
        assert_eq!(block.label(), "20 × 10 × 2 magnet");
        assert_eq!(block.ops().len(), 1);
    }
}
