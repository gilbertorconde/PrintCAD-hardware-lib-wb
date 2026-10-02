//! T-slot aluminium extrusions, as Misumi draws its 5, 6 and 8 series: a
//! block of square cells of one series (20, 30 or 40 mm), a slot on every
//! open outer face of every cell, a hole down each cell, hollows between
//! the cells of a multi-cell profile, and the corner holes or hollows a
//! series has. It runs along X, Y or Z from the origin.

use printcad_bench_sdk::Value;
use printcad_bench_sdk::api::kernel_api::{BooleanOp, ProfilePlane, ProfileSegment, SolidOp};
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

/// Which outer faces carry a slot: every one, or the maker's closed-face
/// variants. A closed face is flat outside and keeps its cavity inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Slots {
    #[default]
    All,
    /// One face flat (the top).
    Three,
    /// Two neighbouring faces flat (top and left).
    Adjacent,
    /// Two facing faces flat (top and bottom).
    Opposite,
    /// One slot (the bottom); three faces flat.
    One,
}

impl Slots {
    pub const ALL: [Slots; 5] = [
        Slots::All,
        Slots::Three,
        Slots::Adjacent,
        Slots::Opposite,
        Slots::One,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Slots::All => "All faces",
            Slots::Three => "Three (top flat)",
            Slots::Adjacent => "Two adjacent (top and left flat)",
            Slots::Opposite => "Two opposite (top and bottom flat)",
            Slots::One => "One (bottom)",
        }
    }

    pub fn named(name: &str) -> Option<Slots> {
        let key = name.to_lowercase();
        Slots::ALL.into_iter().find(|s| {
            s.name().to_lowercase().starts_with(&key)
                || format!("{s:?}").eq_ignore_ascii_case(name)
                || (key == "4" && *s == Slots::All)
                || (key == "3" && *s == Slots::Three)
                || (key == "1" && *s == Slots::One)
        })
    }

    /// Whether side `side` is open: 0 the bottom, 1 the right, 2 the
    /// top, 3 the left.
    pub fn open(self, side: usize) -> bool {
        match self {
            Slots::All => true,
            Slots::Three => side != 2,
            Slots::Adjacent => side == 0 || side == 1,
            Slots::Opposite => side == 0 || side == 2,
            Slots::One => side == 0,
        }
    }
}

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
    #[serde(default)]
    pub slots: Slots,
    pub opening: f64,
    pub lip: f64,
    pub cavity: f64,
    /// Where the cavity's walls turn in at 45° toward the floor.
    #[serde(default)]
    pub shoulder: f64,
    pub depth: f64,
    /// The floor's width; 0 for what 45° walls from the shoulder leave.
    #[serde(default)]
    pub floor: f64,
    pub hole: f64,
    pub corner: f64,
    #[serde(default)]
    pub custom: bool,
}

/// A side of the section: the corner it leaves, the way it runs, its
/// outward normal and how many cells it passes.
type Side = ([f64; 2], [f64; 2], [f64; 2], u32);

/// The section as it is cut: the outline, the round holes `(centre,
/// diameter)` and the hollows, each a closed polygon.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Section {
    pub outline: Vec<[f64; 2]>,
    pub circles: Vec<([f64; 2], f64)>,
    pub hollows: Vec<Vec<[f64; 2]>>,
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
            slots: Slots::All,
            opening: 0.0,
            lip: 0.0,
            cavity: 0.0,
            shoulder: 0.0,
            depth: 0.0,
            floor: 0.0,
            hole: 0.0,
            corner: 0.0,
            custom: false,
        };
        e.refill();
        e
    }

    /// The series it is cut from: the V-slot's when it has V lips.
    pub fn table(&self) -> &'static ExtrusionSeries {
        standards::extrusion_series(self.series, self.v_slot).unwrap_or(&standards::EXTRUSIONS[0])
    }

    pub fn refill(&mut self) {
        let t = *self.table();
        self.series = t.cell;
        self.opening = t.opening;
        self.lip = t.lip;
        self.cavity = t.cavity;
        self.shoulder = t.shoulder;
        self.depth = t.depth;
        self.floor = t.floor;
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

    /// The shoulder's depth, midway down the cavity when none was saved.
    fn shoulder(&self) -> f64 {
        if self.shoulder > self.lip {
            self.shoulder
        } else {
            (self.lip + self.depth) / 2.0
        }
    }

    /// The cavity's floor width: as set, or where 45° walls from the
    /// shoulder end.
    fn floor(&self) -> f64 {
        if self.floor > 0.0 {
            self.floor
        } else {
            self.cavity - 2.0 * (self.depth - self.shoulder())
        }
    }

    /// The core a cell keeps between its cavity floors.
    fn core(&self) -> f64 {
        self.cell() - 2.0 * self.depth
    }

    /// The centre of cell `(i, j)`.
    fn centre(&self, i: u32, j: u32) -> [f64; 2] {
        let c = self.cell();
        [
            -self.width() / 2.0 + c * (i as f64 + 0.5),
            -self.height() / 2.0 + c * (j as f64 + 0.5),
        ]
    }

    /// The four sides, counter-clockwise from the bottom-left corner:
    /// the corner each leaves, the way it runs, its outward normal and
    /// how many cells it passes.
    fn sides(&self) -> [Side; 4] {
        let (w, h) = (self.width() / 2.0, self.height() / 2.0);
        [
            ([-w, -h], [1.0, 0.0], [0.0, -1.0], self.cells_x),
            ([w, -h], [0.0, 1.0], [1.0, 0.0], self.cells_y),
            ([w, h], [-1.0, 0.0], [0.0, 1.0], self.cells_x),
            ([-w, h], [0.0, -1.0], [-1.0, 0.0], self.cells_y),
        ]
    }

    /// The section, every loop counter-clockwise.
    pub fn section(&self) -> Section {
        let add = |a: [f64; 2], b: [f64; 2], k: f64| [a[0] + b[0] * k, a[1] + b[1] * k];
        let (c, t) = (self.cell(), *self.table());
        let r = self.corner.min(c / 4.0);
        let (o, lip, cav, shoulder, depth, floor) = (
            self.opening / 2.0,
            self.lip,
            self.cavity / 2.0,
            self.shoulder(),
            self.depth,
            self.floor() / 2.0,
        );
        let sides = self.sides();
        let mut section = Section::default();
        for (i, (corner, tan, nrm, cells)) in sides.iter().enumerate() {
            let next = sides[(i + 1) % 4].0;
            let open = self.slots.open(i);
            section.outline.push(add(*corner, *tan, r));
            for cell in 0..*cells {
                let centre = add(*corner, *tan, c * (cell as f64 + 0.5));
                let at = |side: f64, deep: f64| add(add(centre, *tan, side), *nrm, -deep);
                // The cavity, from its lip down: at its widest to the
                // shoulder, then in at 45° to the floor.
                let cavity = [
                    at(-cav, lip),
                    at(-cav, shoulder),
                    at(-floor, depth),
                    at(floor, depth),
                    at(cav, shoulder),
                    at(cav, lip),
                ];
                if open {
                    let (lead_in, lead_out) = if self.v_slot {
                        (at(-(o + lip), 0.0), at(o + lip, 0.0))
                    } else {
                        (at(-o, 0.0), at(o, 0.0))
                    };
                    section.outline.push(lead_in);
                    section.outline.push(at(-o, lip));
                    section.outline.extend(cavity);
                    section.outline.push(at(o, lip));
                    section.outline.push(lead_out);
                } else {
                    // A closed face keeps the cavity inside it.
                    section.hollows.push(ccw(cavity.to_vec()));
                }
            }
            let end = add(next, *tan, -r);
            section.outline.push(end);
            if r > 0.0 {
                let t_next = sides[(i + 1) % 4].1;
                let centre = add(add(next, *tan, -r), t_next, r);
                let a0 = (end[1] - centre[1]).atan2(end[0] - centre[0]);
                let a1 = a0 + std::f64::consts::FRAC_PI_2;
                for arc in geom::arc_points(centre, r, a0, a1, 4)
                    .into_iter()
                    .skip(1)
                    .take(3)
                {
                    section.outline.push(arc);
                }
            }
        }
        // A hole down each cell.
        if self.hole > 0.0 {
            for i in 0..self.cells_x {
                for j in 0..self.cells_y {
                    section.circles.push((self.centre(i, j), self.hole));
                }
            }
        }
        // What the series puts in its corner blocks.
        let (w, h) = (self.width() / 2.0, self.height() / 2.0);
        for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            if t.corner_hole > 0.0 {
                let d = t.corner_hole_in;
                section
                    .circles
                    .push(([sx * (w - d), sy * (h - d)], t.corner_hole));
            }
            if t.corner_tri > 0.0 {
                let (a, b) = (self.lip, self.lip + t.corner_tri);
                section.hollows.push(ccw(vec![
                    [sx * (w - a), sy * (h - a)],
                    [sx * (w - a), sy * (h - b)],
                    [sx * (w - b), sy * (h - a)],
                ]));
            }
            if t.corner_void > 0.0 {
                let (a, b) = (t.corner_wall, t.corner_wall + t.corner_void);
                let (x0, x1) = (sx * (w - a), sx * (w - b));
                let (y0, y1) = (sy * (h - a), sy * (h - b));
                section.hollows.push(ccw(vec![
                    [x0.min(x1), y0.min(y1)],
                    [x0.max(x1), y0.min(y1)],
                    [x0.max(x1), y0.max(y1)],
                    [x0.min(x1), y0.max(y1)],
                ]));
            }
        }
        // The hollows between the cells of a multi-cell profile: between
        // neighbouring cores, from core face to core face, kept a web
        // back from any outer face's cavity floor; where four cores meet,
        // the square between them. They join into one hollow where they
        // touch.
        let half = self.core() / 2.0;
        let web = t.web;
        let mut rects: Vec<[f64; 4]> = Vec::new();
        for i in 0..self.cells_x {
            for j in 0..self.cells_y {
                let a = self.centre(i, j);
                if i + 1 < self.cells_x {
                    let b = self.centre(i + 1, j);
                    let (y0, y1) = (
                        a[1] - half + if j == 0 { web } else { 0.0 },
                        a[1] + half - if j + 1 == self.cells_y { web } else { 0.0 },
                    );
                    rects.push([a[0] + half, y0, b[0] - half, y1]);
                }
                if j + 1 < self.cells_y {
                    let b = self.centre(i, j + 1);
                    let (x0, x1) = (
                        a[0] - half + if i == 0 { web } else { 0.0 },
                        a[0] + half - if i + 1 == self.cells_x { web } else { 0.0 },
                    );
                    rects.push([x0, a[1] + half, x1, b[1] - half]);
                }
                if i + 1 < self.cells_x && j + 1 < self.cells_y {
                    let b = self.centre(i + 1, j + 1);
                    rects.push([a[0] + half, a[1] + half, b[0] - half, b[1] - half]);
                }
            }
        }
        section.hollows.extend(union_outlines(&rects));
        section
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
        let slot = standards::extrusion(cell).map_or(0.0, |s| s.opening);
        format!(
            "{cell} series, slot {} ({cell}{cell}, {cell}{}…)",
            fmt(slot),
            2 * cell
        )
    }
}

/// `points` turned counter-clockwise.
fn ccw(mut points: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    if area(&points) < 0.0 {
        points.reverse();
    }
    points
}

/// The signed area of a polygon: positive counter-clockwise.
pub fn area(points: &[[f64; 2]]) -> f64 {
    let n = points.len();
    (0..n)
        .map(|i| {
            let (p, q) = (points[i], points[(i + 1) % n]);
            p[0] * q[1] - q[0] * p[1]
        })
        .sum::<f64>()
        / 2.0
}

/// The outlines of the union of axis-aligned rectangles `[x0, y0, x1,
/// y1]`, one counter-clockwise loop per connected piece. Rectangles that
/// overlap or share an edge join; pieces that only touch at a corner do
/// not come out right, so callers keep clear of that.
pub fn union_outlines(rects: &[[f64; 4]]) -> Vec<Vec<[f64; 2]>> {
    if rects.is_empty() {
        return Vec::new();
    }
    let mut xs: Vec<f64> = rects.iter().flat_map(|r| [r[0], r[2]]).collect();
    let mut ys: Vec<f64> = rects.iter().flat_map(|r| [r[1], r[3]]).collect();
    let dedup = |v: &mut Vec<f64>| {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    };
    dedup(&mut xs);
    dedup(&mut ys);
    let covered = |i: usize, j: usize| -> bool {
        let (cx, cy) = ((xs[i] + xs[i + 1]) / 2.0, (ys[j] + ys[j + 1]) / 2.0);
        rects
            .iter()
            .any(|r| cx > r[0] && cx < r[2] && cy > r[1] && cy < r[3])
    };
    // Every boundary edge, directed with the covered side on its left.
    let mut edges: Vec<([f64; 2], [f64; 2])> = Vec::new();
    for i in 0..xs.len() - 1 {
        for j in 0..ys.len() - 1 {
            if !covered(i, j) {
                continue;
            }
            let (x0, x1, y0, y1) = (xs[i], xs[i + 1], ys[j], ys[j + 1]);
            if j == 0 || !covered(i, j - 1) {
                edges.push(([x0, y0], [x1, y0]));
            }
            if i + 2 == xs.len() || !covered(i + 1, j) {
                edges.push(([x1, y0], [x1, y1]));
            }
            if j + 2 == ys.len() || !covered(i, j + 1) {
                edges.push(([x1, y1], [x0, y1]));
            }
            if i == 0 || !covered(i - 1, j) {
                edges.push(([x0, y1], [x0, y0]));
            }
        }
    }
    // Chain them into loops, dropping the vertices where a loop runs
    // straight on.
    let same = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9;
    let mut loops = Vec::new();
    while let Some(first) = edges.pop() {
        let mut points = vec![first.0, first.1];
        loop {
            let last = *points.last().unwrap_or(&first.1);
            if same(last, first.0) {
                points.pop();
                break;
            }
            let Some(k) = edges.iter().position(|e| same(e.0, last)) else {
                break;
            };
            let e = edges.swap_remove(k);
            points.push(e.1);
        }
        let n = points.len();
        let straight: Vec<[f64; 2]> = (0..n)
            .filter(|&k| {
                let (p, q, r) = (points[(k + n - 1) % n], points[k], points[(k + 1) % n]);
                ((q[0] - p[0]) * (r[1] - q[1]) - (q[1] - p[1]) * (r[0] - q[0])).abs() > 1e-9
            })
            .map(|k| points[k])
            .collect();
        if straight.len() >= 4 {
            loops.push(straight);
        }
    }
    loops
}

impl Part for Extrusion {
    const FAMILY: Family = Family::Extrusion;

    fn label(&self) -> String {
        let name = format!("{}{}", fmt(self.width()), fmt(self.height()));
        let slots = match self.slots {
            Slots::All => String::new(),
            other => format!(", {}", other.name().to_lowercase()),
        };
        format!("{name} × {}{slots}", fmt(self.length))
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
        if self.cells_x >= 3 && self.cells_y >= 3 {
            return Some(
                "A profile three cells wide and three high would close a core inside its hollow; keep one way to two cells."
                    .into(),
            );
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
        if self.hole < 0.0 || self.hole >= self.core() {
            return Some("The centre hole runs into the slots.".into());
        }
        let shoulder = self.shoulder();
        if shoulder <= self.lip || shoulder >= self.depth {
            return Some("The shoulder must lie between the lip and the floor.".into());
        }
        if self.floor() < 1.0 || self.floor() > self.cavity {
            return Some(
                "The cavity's walls meet before its floor: a deeper shoulder or a wider cavity."
                    .into(),
            );
        }
        if self.cavity / 2.0 >= c / 2.0 - shoulder {
            return Some("The cavities of neighbouring faces run into each other.".into());
        }
        let v = if self.v_slot { self.lip } else { 0.0 };
        if self.opening / 2.0 + v + self.corner >= c / 2.0 {
            return Some("The slot opening runs into the corner.".into());
        }
        let t = self.table();
        if t.corner_void > 0.0 && t.corner_wall + t.corner_void >= (c - self.cavity) / 2.0 {
            return Some("The corner hollow runs into the slots.".into());
        }
        if (self.cells_x > 1 || self.cells_y > 1) && self.core() <= 2.0 * t.web + 0.5 {
            return Some("The core is too small to leave a hollow between the cells.".into());
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
        let section = self.section();
        let mut wires: Vec<Vec<ProfileSegment>> = vec![geom::polygon(&section.outline)];
        wires.extend(
            section
                .circles
                .iter()
                .map(|(c, d)| vec![geom::circle_at(*c, *d)]),
        );
        wires.extend(section.hollows.iter().map(|h| geom::polygon(h)));
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
        let slots: Vec<&str> = Slots::ALL.iter().map(|s| s.name()).collect();
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
            choice(
                "slots",
                "Slotted faces",
                &slots,
                Slots::ALL
                    .iter()
                    .position(|s| *s == self.slots)
                    .unwrap_or(0),
            ),
            toggle("v_slot", "V-slot (OpenBuilds)", self.v_slot),
        ];
        let mut dims = vec![toggle("custom", "Custom slot", self.custom)];
        if self.custom {
            dims.push(number(ctx, "opening", "Slot opening", self.opening, 0.1, 2));
            dims.push(number(ctx, "lip", "Lip thickness", self.lip, 0.1, 2));
            dims.push(number(ctx, "cavity", "Cavity width", self.cavity, 0.1, 2));
            dims.push(number(
                ctx,
                "shoulder",
                "Shoulder depth",
                self.shoulder(),
                0.1,
                2,
            ));
            dims.push(number(ctx, "depth", "Slot depth", self.depth, 0.1, 2));
            dims.push(number(ctx, "floor", "Floor width", self.floor(), 0.1, 2));
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
            length("shoulder", "Shoulder depth"),
            length("depth", "Slot depth"),
            length("floor", "Floor width"),
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
                "slots" => self.slots = Slots::ALL.get(*index).copied().unwrap_or_default(),
                _ => return false,
            },
            PanelEvent::Number { id, value } => match id.as_str() {
                "cells_x" => self.cells_x = value.round().max(1.0) as u32,
                "cells_y" => self.cells_y = value.round().max(1.0) as u32,
                "length" => self.length = *value,
                "opening" => self.opening = *value,
                "lip" => self.lip = *value,
                "cavity" => self.cavity = *value,
                "shoulder" => self.shoulder = *value,
                "depth" => self.depth = *value,
                "floor" => self.floor = *value,
                "hole" => self.hole = *value,
                "corner" => self.corner = *value,
                _ => return false,
            },
            PanelEvent::Toggle { id, on } => match id.as_str() {
                "v_slot" => {
                    self.v_slot = *on;
                    if !self.custom {
                        self.refill();
                    }
                }
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
        if let Some(slots) = arg_str(args, "slots") {
            e.slots = Slots::named(slots).ok_or_else(|| {
                format!("`{slots}` is not a slot layout: all, three, adjacent, opposite or one")
            })?;
        }
        if let Some(on) = arg_bool(args, "v_slot") {
            e.v_slot = on;
            e.refill();
        }
        for (key, slot) in [
            ("opening", &mut e.opening),
            ("lip", &mut e.lip),
            ("cavity", &mut e.cavity),
            ("shoulder", &mut e.shoulder),
            ("depth", &mut e.depth),
            ("floor", &mut e.floor),
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
        let section = self.section();
        let (w, h) = (self.width() / 2.0, self.height() / 2.0);
        let off = Sketch::standoff(self.width().max(self.height()));
        s.poly(&section.outline, DiagramStroke::Outline, false);
        for (c, d) in &section.circles {
            s.circle(*c, *d, DiagramStroke::Outline, false);
        }
        for hollow in &section.hollows {
            s.poly(hollow, DiagramStroke::Outline, false);
        }
        for i in 0..self.cells_x {
            for j in 0..self.cells_y {
                let c = self.centre(i, j);
                let k = self.cell() * 0.1;
                s.line(&[[c[0] - k, c[1]], [c[0] + k, c[1]]], DiagramStroke::Axis);
                s.line(&[[c[0], c[1] - k], [c[0], c[1] + k]], DiagramStroke::Axis);
            }
        }
        s.width("cells_x", -w, w, h, off, fmt(self.width()));
        s.height("cells_y", -w, -h, h, off, fmt(self.height()));
        // The slot on the bottom of the first cell is measured.
        let cx = -w + self.cell() / 2.0;
        let o = self.opening / 2.0;
        s.width("opening", cx - o, cx + o, -h, -off, fmt(self.opening));
        let cav = self.cavity / 2.0;
        s.hidden(&[[cx - cav, -h + self.shoulder()], [cx - cav, -h + self.lip]]);
        s.callout(
            "depth",
            [cx + self.floor() / 2.0, -h + self.depth],
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
            let c = self.centre(0, 0);
            s.callout(
                "hole",
                [c[0] + self.hole / 2.0, c[1]],
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

    /// Whether segments `a`–`b` and `c`–`d` cross.
    fn crosses(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
        let orient = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| {
            (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
        };
        let (o1, o2) = (orient(a, b, c), orient(a, b, d));
        let (o3, o4) = (orient(c, d, a), orient(c, d, b));
        o1 * o2 < -1e-12 && o3 * o4 < -1e-12
    }

    fn simple(points: &[[f64; 2]]) -> bool {
        let n = points.len();
        for i in 0..n {
            for j in i + 2..n {
                if i == 0 && j == n - 1 {
                    continue;
                }
                if crosses(
                    points[i],
                    points[(i + 1) % n],
                    points[j],
                    points[(j + 1) % n],
                ) {
                    return false;
                }
            }
        }
        true
    }

    /// The section's area: the outline less its holes and hollows.
    fn section_area(e: &Extrusion) -> f64 {
        let s = e.section();
        area(&s.outline)
            - s.circles
                .iter()
                .map(|(_, d)| std::f64::consts::PI * d * d / 4.0)
                .sum::<f64>()
            - s.hollows.iter().map(|h| area(h).abs()).sum::<f64>()
    }

    fn inside(p: [f64; 2], poly: &[[f64; 2]]) -> bool {
        let n = poly.len();
        let mut hit = false;
        for i in 0..n {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            if (a[1] > p[1]) != (b[1] > p[1])
                && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
            {
                hit = !hit;
            }
        }
        hit
    }

    #[test]
    fn a_2020_is_sized_as_its_maker_draws_it() {
        let e = Extrusion::new(20, 1, 1, 100.0);
        assert_eq!(e.label(), "2020 × 100");
        assert_eq!(e.problem(), None);
        let s = e.section();
        assert_eq!(s.circles, [([0.0, 0.0], 4.2)]);
        assert!(s.hollows.is_empty());
        assert!(simple(&s.outline));
        assert!(
            (e.floor() - 6.0).abs() < 1e-9,
            "the floor, which a 5.8 nut sits on"
        );
        assert!((e.core() - 8.0).abs() < 1e-9);
        // The catalogue says 183 mm².
        let a = section_area(&e);
        assert!((a - 183.0).abs() < 183.0 * 0.03, "{a}");
    }

    #[test]
    fn every_series_outline_is_simple_and_weighs_what_the_catalogue_says() {
        for series in standards::EXTRUSIONS.iter().chain([&standards::VSLOT_20]) {
            let mut e = Extrusion::new(series.cell, 1, 1, 10.0);
            e.v_slot = series.floor > 0.0;
            e.refill();
            assert_eq!(e.problem(), None, "{}", series.cell);
            let s = e.section();
            assert!(simple(&s.outline), "{}", series.cell);
            for h in &s.hollows {
                assert!(simple(h), "{}", series.cell);
                assert!(
                    area(h) > 0.0,
                    "{}: hollows are counter-clockwise",
                    series.cell
                );
            }
            let a = section_area(&e);
            assert!(
                (a - series.area_mm2).abs() < series.area_mm2 * 0.12,
                "{}: {a} against {}",
                series.cell,
                series.area_mm2
            );
        }
    }

    #[test]
    fn a_2040_is_hollow_between_its_cells_and_a_4040_in_a_cross() {
        let e = Extrusion::new(20, 1, 2, 50.0);
        assert_eq!(e.label(), "2040 × 50");
        let s = e.section();
        assert_eq!(s.circles.len(), 2);
        assert_eq!(s.hollows.len(), 1, "one hollow between the two cells");
        let h = &s.hollows[0];
        assert!(inside([0.0, 0.0], h));
        assert!(!inside([3.0, 0.0], h), "a web keeps it from the side slots");
        assert!(!inside([0.0, 7.0], h), "it stops at the core");
        let e = Extrusion::new(20, 2, 2, 50.0);
        let s = e.section();
        assert_eq!(s.circles.len(), 4);
        assert_eq!(s.hollows.len(), 1, "the channels and the square join");
        let h = &s.hollows[0];
        assert!(inside([0.0, 0.0], h));
        assert!(inside([0.0, 10.0], h), "12 wide at the hole row, as drawn");
        assert!(inside([5.5, 10.0], h));
        assert!(
            !inside([0.0, 13.0], h),
            "a web under the top face's cavities"
        );
        assert!(!inside([8.0, 8.0], h), "the cores are solid");
        assert!(simple(h));
        assert_eq!(e.ops().len(), 1);
    }

    #[test]
    fn closed_faces_keep_their_cavities_inside() {
        let mut e = Extrusion::new(20, 1, 1, 10.0);
        e.slots = Slots::Three;
        let s = e.section();
        assert_eq!(s.hollows.len(), 1, "the top's cavity, closed");
        assert!(inside([0.0, 6.0], &s.hollows[0]));
        assert!(s.outline.iter().all(|p| p[1] < 10.0 + 1e-9));
        assert!(simple(&s.outline));
        e.slots = Slots::One;
        assert_eq!(e.section().hollows.len(), 3);
        assert_eq!(e.label(), "2020 × 10, one (bottom)");
    }

    #[test]
    fn the_30_series_has_corner_holes_and_the_40_series_hollow_corners() {
        let s = Extrusion::new(30, 1, 1, 10.0).section();
        assert_eq!(s.circles.len(), 5);
        assert!(
            s.circles
                .iter()
                .any(|(c, d)| *d == 4.2 && (c[0] - 11.6).abs() < 1e-9)
        );
        let s = Extrusion::new(40, 1, 1, 10.0).section();
        assert_eq!(s.hollows.len(), 4);
        assert!(s.hollows.iter().any(|h| inside([15.0, 15.0], h)));
    }

    #[test]
    fn a_v_slot_is_openbuilds_profile() {
        let mut e = Extrusion::new(20, 1, 1, 10.0);
        let plain = e.section();
        e.v_slot = true;
        e.refill();
        assert_eq!(
            (e.opening, e.lip, e.cavity, e.depth, e.floor),
            (5.68, 1.8, 11.0, 6.1, 5.68)
        );
        assert_eq!(e.problem(), None);
        let s = e.section();
        assert_ne!(plain.outline, s.outline);
        assert!(simple(&s.outline));
        // The V opens 9.28 at the surface and the corners are hollow.
        assert!(
            s.outline
                .iter()
                .any(|p| (p[1] + 10.0).abs() < 1e-9 && (p[0].abs() - 4.64).abs() < 1e-9)
        );
        assert_eq!(s.hollows.len(), 4);
        let a = section_area(&e);
        assert!((a - 164.0).abs() < 164.0 * 0.03, "{a}");
        e.v_slot = false;
        e.refill();
        assert_eq!(e.section().outline, plain.outline, "back to the T-slot");
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
            &json!({"profile": "4080", "series": 40, "along": "y", "slots": "three"}),
            &Defaults::default(),
        )
        .unwrap();
        assert_eq!(
            (e.cells_x, e.cells_y, e.along.as_str(), e.slots),
            (1, 2, "Y", Slots::Three)
        );
        assert!(Extrusion::with_args(&json!({"profile": "2030"}), &Defaults::default()).is_err());
    }

    #[test]
    fn rectangles_union_into_outlines() {
        let plus = union_outlines(&[
            [-6.0, -6.0, 6.0, 6.0],
            [-6.0, 6.0, 6.0, 12.0],
            [6.0, -6.0, 12.0, 6.0],
        ]);
        assert_eq!(plus.len(), 1);
        assert_eq!(plus[0].len(), 6, "the collinear bottom edge is one");
        assert!((area(&plus[0]) - (144.0 + 72.0 + 72.0)).abs() < 1e-9);
        let two = union_outlines(&[[0.0, 0.0, 1.0, 1.0], [2.0, 0.0, 3.0, 1.0]]);
        assert_eq!(two.len(), 2);
    }

    #[test]
    fn a_slot_into_the_centre_hole_and_a_three_by_three_are_refused() {
        let mut e = Extrusion::new(20, 1, 1, 10.0);
        e.depth = 9.0;
        assert!(e.problem().is_some());
        assert!(Extrusion::new(20, 3, 3, 10.0).problem().is_some());
    }
}
