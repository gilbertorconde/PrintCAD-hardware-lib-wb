//! T-slot aluminium extrusions: a block of square cells of one series
//! (20, 30 or 40 mm), a slot on every outer face of every cell and a
//! hole down each cell, run along X, Y or Z from the origin.

use printcad_bench_sdk::Value;
use printcad_bench_sdk::api::kernel_api::{BooleanOp, ProfilePlane, SolidOp};
use printcad_bench_sdk::api::{DiagramStroke, PanelEvent, Parameter, Widget};
use serde::{Deserialize, Serialize};

use super::{
    Ctx, Defaults, Family, Part, arg_bool, arg_f64, arg_str, choice, count, fmt, group, integer,
    length, note, number, toggle,
};
use crate::diagram::Sketch;
use crate::geom;
use crate::standards::{self, ExtrusionSeries};

pub const AXES: [&str; 3] = ["X", "Y", "Z"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Extrusion {
    /// The cell size: 20, 30 or 40.
    pub series: u32,
    pub cells_x: u32,
    pub cells_y: u32,
    pub length: f64,
    /// The axis it runs along: `X`, `Y` or `Z`.
    pub along: String,
    /// The slot's lips are cut back at 45°, for wheels to ride on.
    #[serde(default)]
    pub v_slot: bool,
    pub opening: f64,
    pub lip: f64,
    pub cavity: f64,
    pub depth: f64,
    pub hole: f64,
    pub corner: f64,
    #[serde(default)]
    pub custom: bool,
}

impl Extrusion {
    pub fn new(series: u32, cells_x: u32, cells_y: u32, length: f64) -> Extrusion {
        let mut e = Extrusion {
            series,
            cells_x,
            cells_y,
            length,
            along: "Z".into(),
            v_slot: false,
            opening: 0.0,
            lip: 0.0,
            cavity: 0.0,
            depth: 0.0,
            hole: 0.0,
            corner: 0.0,
            custom: false,
        };
        e.refill();
        e
    }

    fn table(&self) -> &'static ExtrusionSeries {
        standards::extrusion(self.series).unwrap_or(&standards::EXTRUSIONS[0])
    }

    pub fn refill(&mut self) {
        let t = *self.table();
        self.series = t.cell;
        self.opening = t.opening;
        self.lip = t.lip;
        self.cavity = t.cavity;
        self.depth = t.depth;
        self.hole = t.hole;
        self.corner = t.corner;
    }

    fn cell(&self) -> f64 {
        self.series as f64
    }

    pub fn width(&self) -> f64 {
        self.cells_x as f64 * self.cell()
    }

    pub fn height(&self) -> f64 {
        self.cells_y as f64 * self.cell()
    }

    /// The section's outline about the origin, counter-clockwise, with a
    /// slot on every outer cell face, and the centre of every hole.
    pub fn outline(&self) -> (Vec<[f64; 2]>, Vec<[f64; 2]>) {
        let (w, h, c) = (self.width() / 2.0, self.height() / 2.0, self.cell());
        let r = self.corner.min(c / 4.0);
        // The four sides, counter-clockwise from the bottom-left corner:
        // the corner it leaves, the way it runs, its outward normal and
        // how many cells it passes.
        type Side = ([f64; 2], [f64; 2], [f64; 2], u32);
        let sides: [Side; 4] = [
            ([-w, -h], [1.0, 0.0], [0.0, -1.0], self.cells_x),
            ([w, -h], [0.0, 1.0], [1.0, 0.0], self.cells_y),
            ([w, h], [-1.0, 0.0], [0.0, 1.0], self.cells_x),
            ([-w, h], [0.0, -1.0], [-1.0, 0.0], self.cells_y),
        ];
        let add = |a: [f64; 2], b: [f64; 2], k: f64| [a[0] + b[0] * k, a[1] + b[1] * k];
        let mut points = Vec::new();
        for (i, (corner, t, n, cells)) in sides.iter().enumerate() {
            let next = sides[(i + 1) % 4].0;
            points.push(add(*corner, *t, r));
            for cell in 0..*cells {
                let centre = add(*corner, *t, c * (cell as f64 + 0.5));
                let (o, lip, cav, depth) =
                    (self.opening / 2.0, self.lip, self.cavity / 2.0, self.depth);
                let lipped = |side: f64, deep: f64| add(add(centre, *t, side), *n, -deep);
                if self.v_slot {
                    points.push(lipped(-(o + lip), 0.0));
                } else {
                    points.push(lipped(-o, 0.0));
                }
                points.push(lipped(-o, lip));
                points.push(lipped(-cav, lip));
                points.push(lipped(-cav, depth));
                points.push(lipped(cav, depth));
                points.push(lipped(cav, lip));
                points.push(lipped(o, lip));
                if self.v_slot {
                    points.push(lipped(o + lip, 0.0));
                } else {
                    points.push(lipped(o, 0.0));
                }
            }
            // Round the corner ahead.
            let end = add(next, *t, -r);
            points.push(end);
            if r > 0.0 {
                let t_next = sides[(i + 1) % 4].1;
                let centre = add(add(next, *t, -r), t_next, r);
                let a0 = (end[1] - centre[1]).atan2(end[0] - centre[0]);
                let a1 = a0 + std::f64::consts::FRAC_PI_2;
                for arc in geom::arc_points(centre, r, a0, a1, 4)
                    .into_iter()
                    .skip(1)
                    .take(3)
                {
                    points.push(arc);
                }
            }
        }
        let mut holes = Vec::new();
        for ix in 0..self.cells_x {
            for iy in 0..self.cells_y {
                holes.push([-w + c * (ix as f64 + 0.5), -h + c * (iy as f64 + 0.5)]);
            }
        }
        (points, holes)
    }

    fn plane(&self) -> ProfilePlane {
        match self.along.as_str() {
            "X" => ProfilePlane {
                origin: [0.0; 3],
                x_axis: [0.0, 1.0, 0.0],
                y_axis: [0.0, 0.0, 1.0],
                normal: [1.0, 0.0, 0.0],
            },
            "Y" => ProfilePlane {
                origin: [0.0; 3],
                x_axis: [0.0, 0.0, 1.0],
                y_axis: [1.0, 0.0, 0.0],
                normal: [0.0, 1.0, 0.0],
            },
            _ => geom::xy(0.0),
        }
    }

    fn series_label(cell: u32) -> String {
        format!("{cell} series ({cell}{cell}, {cell}{}…)", 2 * cell)
    }
}

impl Part for Extrusion {
    const FAMILY: Family = Family::Extrusion;

    fn label(&self) -> String {
        format!(
            "{}{} × {}",
            fmt(self.width()),
            fmt(self.height()),
            fmt(self.length)
        )
    }

    fn icon(&self) -> &'static str {
        "extrusion"
    }

    fn problem(&self) -> Option<String> {
        if self.length <= 0.0 || self.cells_x == 0 || self.cells_y == 0 {
            return Some("The length and the cells must be more than 0.".into());
        }
        if self.cells_x > 8 || self.cells_y > 8 {
            return Some("At most 8 cells each way.".into());
        }
        let c = self.cell();
        if self.opening <= 0.0 || self.cavity <= self.opening || self.cavity >= c - 1.0 {
            return Some(
                "The cavity must be wider than the opening and narrower than the cell.".into(),
            );
        }
        if self.lip <= 0.0 || self.depth <= self.lip || self.depth >= c / 2.0 - self.hole / 2.0 {
            return Some(
                "The slot must be deeper than its lip and clear of the centre hole.".into(),
            );
        }
        if self.hole < 0.0 || self.hole >= c - 2.0 * self.depth {
            return Some("The centre hole runs into the slots.".into());
        }
        let v = if self.v_slot { self.lip } else { 0.0 };
        if self.opening / 2.0 + v + self.corner >= c / 2.0 {
            return Some("The slot opening runs into the corner.".into());
        }
        if !AXES.contains(&self.along.as_str()) {
            return Some("The axis is X, Y or Z.".into());
        }
        None
    }

    fn axis(&self) -> ([f64; 3], [f64; 3]) {
        let n = self.plane().normal;
        (
            [0.0; 3],
            [n[0] * self.length, n[1] * self.length, n[2] * self.length],
        )
    }

    fn ops(&self) -> Vec<SolidOp> {
        let (outline, holes) = self.outline();
        let mut wires = vec![geom::polygon(&outline)];
        if self.hole > 0.0 {
            wires.extend(
                holes
                    .into_iter()
                    .map(|c| vec![geom::circle_at(c, self.hole)]),
            );
        }
        vec![geom::extrude(
            self.plane(),
            wires,
            self.length,
            BooleanOp::NewSolid,
        )]
    }

    fn panel(&self, ctx: &Ctx) -> Vec<Widget> {
        let series: Vec<String> = standards::EXTRUSIONS
            .iter()
            .map(|s| Self::series_label(s.cell))
            .collect();
        let selected = standards::EXTRUSIONS
            .iter()
            .position(|s| s.cell == self.series)
            .unwrap_or(0);
        let mut widgets = vec![
            self.drawing(ctx),
            choice("series", "Series", &series, selected),
            count(ctx, "cells_x", "Cells across", self.cells_x as f64, 1.0),
            count(ctx, "cells_y", "Cells up", self.cells_y as f64, 1.0),
            number(ctx, "length", "Length", self.length, 1.0, 1),
            choice(
                "along",
                "Along",
                &AXES,
                AXES.iter().position(|a| *a == self.along).unwrap_or(2),
            ),
            toggle("v_slot", "V-slot", self.v_slot),
        ];
        let mut dims = vec![toggle("custom", "Custom slot", self.custom)];
        if self.custom {
            dims.push(number(ctx, "opening", "Slot opening", self.opening, 0.1, 2));
            dims.push(number(ctx, "lip", "Lip thickness", self.lip, 0.1, 2));
            dims.push(number(ctx, "cavity", "Cavity width", self.cavity, 0.1, 2));
            dims.push(number(ctx, "depth", "Slot depth", self.depth, 0.1, 2));
            dims.push(number(ctx, "hole", "Centre hole", self.hole, 0.0, 2));
            dims.push(number(ctx, "corner", "Corner radius", self.corner, 0.0, 2));
        }
        widgets.push(group("Slot", self.custom, dims));
        widgets.extend(note(self.problem()));
        widgets
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            integer("cells_x", "Cells across"),
            integer("cells_y", "Cells up"),
            length("length", "Length"),
            length("opening", "Slot opening"),
            length("lip", "Lip"),
            length("cavity", "Cavity"),
            length("depth", "Slot depth"),
            length("hole", "Centre hole"),
            length("corner", "Corner radius"),
        ]
    }

    fn apply(&mut self, event: &PanelEvent) -> bool {
        match event {
            PanelEvent::Choice { id, index } => match id.as_str() {
                "series" => {
                    self.series = standards::EXTRUSIONS.get(*index).map_or(20, |s| s.cell);
                    if !self.custom {
                        self.refill();
                    }
                }
                "along" => self.along = AXES.get(*index).unwrap_or(&"Z").to_string(),
                _ => return false,
            },
            PanelEvent::Number { id, value } => match id.as_str() {
                "cells_x" => self.cells_x = value.round().max(1.0) as u32,
                "cells_y" => self.cells_y = value.round().max(1.0) as u32,
                "length" => self.length = *value,
                "opening" => self.opening = *value,
                "lip" => self.lip = *value,
                "cavity" => self.cavity = *value,
                "depth" => self.depth = *value,
                "hole" => self.hole = *value,
                "corner" => self.corner = *value,
                _ => return false,
            },
            PanelEvent::Toggle { id, on } => match id.as_str() {
                "v_slot" => self.v_slot = *on,
                "custom" => {
                    self.custom = *on;
                    if !*on {
                        self.refill();
                    }
                }
                _ => return false,
            },
            _ => return false,
        }
        true
    }

    fn with_args(args: &Value, defaults: &Defaults) -> Result<Self, String> {
        let series = arg_f64(args, "series").map_or(defaults.series, |s| s as u32);
        if standards::extrusion(series).is_none() {
            return Err(format!(
                "`{series}` is not a series: {}",
                standards::EXTRUSIONS.map(|s| s.cell.to_string()).join(", ")
            ));
        }
        // `profile = "2040"` names the cells; `cells_x` and `cells_y` do too.
        let (mut cx, mut cy) = (1, 1);
        if let Some(profile) = arg_str(args, "profile") {
            let digits = profile.trim();
            let half = digits.len() / 2;
            let (a, b) = (
                digits[..half].parse::<u32>().ok(),
                digits[half..].parse::<u32>().ok(),
            );
            match (a, b) {
                (Some(a), Some(b)) if a % series == 0 && b % series == 0 && a > 0 && b > 0 => {
                    cx = a / series;
                    cy = b / series;
                }
                _ => {
                    return Err(format!(
                        "`{profile}` is not a profile of the {series} series, such as {series}{series}"
                    ));
                }
            }
        }
        if let Some(v) = arg_f64(args, "cells_x") {
            cx = v.round().max(1.0) as u32;
        }
        if let Some(v) = arg_f64(args, "cells_y") {
            cy = v.round().max(1.0) as u32;
        }
        let length = arg_f64(args, "length").unwrap_or(100.0);
        let mut e = Extrusion::new(series, cx, cy, length);
        if let Some(along) = arg_str(args, "along") {
            let along = along.to_uppercase();
            if !AXES.contains(&along.as_str()) {
                return Err(format!("`{along}` is not an axis: X, Y or Z"));
            }
            e.along = along;
        }
        if let Some(on) = arg_bool(args, "v_slot") {
            e.v_slot = on;
        }
        for (key, slot) in [
            ("opening", &mut e.opening),
            ("lip", &mut e.lip),
            ("cavity", &mut e.cavity),
            ("depth", &mut e.depth),
            ("hole", &mut e.hole),
            ("corner", &mut e.corner),
        ] {
            if let Some(v) = arg_f64(args, key) {
                *slot = v;
                e.custom = true;
            }
        }
        Ok(e)
    }
}

impl Extrusion {
    /// The section, as it is cut.
    fn drawing(&self, ctx: &Ctx) -> Widget {
        let mut s = Sketch::new(ctx.focus);
        let (outline, holes) = self.outline();
        let (w, h) = (self.width() / 2.0, self.height() / 2.0);
        let off = Sketch::standoff(self.width().max(self.height()));
        s.poly(&outline, DiagramStroke::Outline, false);
        for hole in &holes {
            if self.hole > 0.0 {
                s.circle(*hole, self.hole, DiagramStroke::Outline, false);
            }
            s.line(
                &[
                    [hole[0] - self.cell() * 0.1, hole[1]],
                    [hole[0] + self.cell() * 0.1, hole[1]],
                ],
                DiagramStroke::Axis,
            );
            s.line(
                &[
                    [hole[0], hole[1] - self.cell() * 0.1],
                    [hole[0], hole[1] + self.cell() * 0.1],
                ],
                DiagramStroke::Axis,
            );
        }
        s.width("cells_x", -w, w, h, off, fmt(self.width()));
        s.height("cells_y", -w, -h, h, off, fmt(self.height()));
        // The slot on the bottom of the first cell is measured.
        let cx = -w + self.cell() / 2.0;
        let o = self.opening / 2.0;
        s.width("opening", cx - o, cx + o, -h, -off, fmt(self.opening));
        let cav = self.cavity / 2.0;
        s.hidden(&[[cx - cav, -h + self.depth], [cx - cav, -h + self.lip]]);
        s.callout(
            "depth",
            [cx + cav, -h + self.depth],
            [w + off * 1.6, -h + off * 1.2],
            format!("depth {}", fmt(self.depth)),
        );
        s.callout(
            "length",
            [w * 0.5, h * 0.5],
            [w + off * 1.6, h + off * 0.2],
            format!("L {} along {}", fmt(self.length), self.along),
        );
        if self.hole > 0.0 {
            s.callout(
                "hole",
                [holes[0][0] + self.hole / 2.0, holes[0][1]],
                [-w - off * 1.6, -h - off * 0.6],
                format!("Ø{}", fmt(self.hole)),
            );
        }
        s.finish("extrusion")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use printcad_bench_sdk::json;

    #[test]
    fn a_2020_has_four_slots_and_one_hole() {
        let e = Extrusion::new(20, 1, 1, 100.0);
        assert_eq!(e.label(), "2020 × 100");
        assert_eq!(e.problem(), None);
        let (outline, holes) = e.outline();
        assert_eq!(holes, [[0.0, 0.0]]);
        // 4 sides × (start, 8 slot points, end, 3 arc points).
        assert_eq!(outline.len(), 4 * 13);
        let xs = outline.iter().map(|p| p[0]);
        assert!(xs.clone().fold(f64::MIN, f64::max) <= 10.0 + 1e-9);
        assert!(xs.fold(f64::MAX, f64::min) >= -10.0 - 1e-9);
        // The slot's floor sits at its depth.
        assert!(outline.iter().any(|p| (p[1] - (-10.0 + 6.0)).abs() < 1e-9));
    }

    #[test]
    fn a_2040_has_six_slots_and_two_holes() {
        let e = Extrusion::new(20, 1, 2, 50.0);
        assert_eq!(e.label(), "2040 × 50");
        let (outline, holes) = e.outline();
        assert_eq!(holes.len(), 2);
        assert_eq!(outline.len(), 4 * 5 + 6 * 8);
        assert_eq!(e.ops().len(), 1);
    }

    #[test]
    fn a_v_slot_cuts_its_lips_back() {
        let mut e = Extrusion::new(20, 1, 1, 10.0);
        let plain = e.outline().0;
        e.v_slot = true;
        let v = e.outline().0;
        assert_eq!(plain.len(), v.len());
        assert_ne!(plain, v);
        assert_eq!(e.problem(), None);
    }

    #[test]
    fn it_runs_along_the_axis_asked() {
        let mut e = Extrusion::new(30, 1, 1, 80.0);
        assert!(e.apply(&PanelEvent::Choice {
            id: "along".into(),
            index: 0,
        }));
        assert_eq!(e.axis().1, [80.0, 0.0, 0.0]);
        let e = Extrusion::with_args(
            &json!({"profile": "4080", "series": 40, "along": "y"}),
            &Defaults::default(),
        )
        .unwrap();
        assert_eq!((e.cells_x, e.cells_y, e.along.as_str()), (1, 2, "Y"));
        assert!(Extrusion::with_args(&json!({"profile": "2030"}), &Defaults::default()).is_err());
    }

    #[test]
    fn a_slot_into_the_centre_hole_is_refused() {
        let mut e = Extrusion::new(20, 1, 1, 10.0);
        e.depth = 9.0;
        assert!(e.problem().is_some());
    }
}
