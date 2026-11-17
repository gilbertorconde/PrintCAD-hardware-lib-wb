//! The drawing at the top of a part's panel: a side view of the part at
//! its true proportions, with its measures marked beside the parts they
//! size. The measure being edited is emphasised.
//!
//! A sketch is drawn in millimetres, y up, anywhere; `finish` fits a
//! margin around everything drawn and hands it to the host as a
//! `Widget::Diagram`, which scales it to the panel.

use printcad_bench_sdk::api::{Callout, DiagramShape, DiagramStroke, Dimension, Widget};

/// A part's drawing as it is built up.
pub struct Sketch<'a> {
    /// The field being edited, whose measure is emphasised.
    focus: Option<&'a str>,
    shapes: Vec<DiagramShape>,
    dimensions: Vec<Dimension>,
    callouts: Vec<Callout>,
    min: [f32; 2],
    max: [f32; 2],
}

/// How far a dimension line stands off the part, as a share of the
/// part's larger extent.
const STANDOFF: f64 = 0.14;

impl<'a> Sketch<'a> {
    pub fn new(focus: Option<&'a str>) -> Self {
        Self {
            focus,
            shapes: Vec::new(),
            dimensions: Vec::new(),
            callouts: Vec::new(),
            min: [f32::MAX; 2],
            max: [f32::MIN; 2],
        }
    }

    fn cover(&mut self, p: [f32; 2]) {
        self.min = [self.min[0].min(p[0]), self.min[1].min(p[1])];
        self.max = [self.max[0].max(p[0]), self.max[1].max(p[1])];
    }

    fn pt(p: [f64; 2]) -> [f32; 2] {
        [p[0] as f32, p[1] as f32]
    }

    /// A closed outline, shaded when `fill` (keep it convex).
    pub fn poly(&mut self, points: &[[f64; 2]], stroke: DiagramStroke, fill: bool) -> &mut Self {
        let points: Vec<[f32; 2]> = points.iter().map(|p| Self::pt(*p)).collect();
        for p in &points {
            self.cover(*p);
        }
        self.shapes.push(DiagramShape::Path {
            points,
            closed: true,
            stroke,
            fill,
        });
        self
    }

    /// A shaded rectangle with an outline, from corner to corner.
    pub fn rect(&mut self, a: [f64; 2], b: [f64; 2]) -> &mut Self {
        self.poly(
            &[a, [b[0], a[1]], b, [a[0], b[1]]],
            DiagramStroke::Outline,
            true,
        )
    }

    /// A run of lines.
    pub fn line(&mut self, points: &[[f64; 2]], stroke: DiagramStroke) -> &mut Self {
        let points: Vec<[f32; 2]> = points.iter().map(|p| Self::pt(*p)).collect();
        for p in &points {
            self.cover(*p);
        }
        self.shapes.push(DiagramShape::Path {
            points,
            closed: false,
            stroke,
            fill: false,
        });
        self
    }

    pub fn circle(
        &mut self,
        center: [f64; 2],
        d: f64,
        stroke: DiagramStroke,
        fill: bool,
    ) -> &mut Self {
        let r = (d / 2.0) as f32;
        let c = Self::pt(center);
        self.cover([c[0] - r, c[1] - r]);
        self.cover([c[0] + r, c[1] + r]);
        self.shapes.push(DiagramShape::Circle {
            center: c,
            radius: r,
            stroke,
            fill,
        });
        self
    }

    /// A centre line through the drawing's height at `x`, from `y0` to `y1`.
    pub fn axis(&mut self, x: f64, y0: f64, y1: f64) -> &mut Self {
        self.line(&[[x, y0], [x, y1]], DiagramStroke::Axis)
    }

    /// Hidden edges: a hole or a socket behind the surface.
    pub fn hidden(&mut self, points: &[[f64; 2]]) -> &mut Self {
        self.line(points, DiagramStroke::Hidden)
    }

    /// A measure from `from` to `to`, its line `offset` to the left of
    /// that way (negative: the right), emphasised while `key` is edited.
    pub fn dim(
        &mut self,
        key: &str,
        from: [f64; 2],
        to: [f64; 2],
        offset: f64,
        text: impl Into<String>,
    ) -> &mut Self {
        let (f, t) = (Self::pt(from), Self::pt(to));
        let dx = t[0] - f[0];
        let dy = t[1] - f[1];
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-6 {
            return self;
        }
        self.dimensions.push(Dimension {
            from: f,
            to: t,
            offset: offset as f32,
            text: text.into(),
            emphasis: self.focus == Some(key),
            ..Default::default()
        });
        self
    }

    /// A horizontal measure under or over the part at height `y`.
    pub fn width(
        &mut self,
        key: &str,
        x0: f64,
        x1: f64,
        y: f64,
        offset: f64,
        text: impl Into<String>,
    ) -> &mut Self {
        self.dim(key, [x0, y], [x1, y], offset, text)
    }

    /// A vertical measure beside the part at `x`.
    pub fn height(
        &mut self,
        key: &str,
        x: f64,
        y0: f64,
        y1: f64,
        offset: f64,
        text: impl Into<String>,
    ) -> &mut Self {
        self.dim(key, [x, y0], [x, y1], offset, text)
    }

    /// A note pointing at `anchor`, its text at `at`.
    pub fn callout(
        &mut self,
        key: &str,
        anchor: [f64; 2],
        at: [f64; 2],
        text: impl Into<String>,
    ) -> &mut Self {
        let a = Self::pt(at);
        self.cover(Self::pt(anchor));
        self.cover(a);
        self.callouts.push(Callout {
            anchor: Self::pt(anchor),
            at: a,
            text: text.into(),
            emphasis: self.focus == Some(key),
            ..Default::default()
        });
        self
    }

    /// The standoff for a drawing of a part about `extent` across.
    pub fn standoff(extent: f64) -> f64 {
        (extent * STANDOFF).max(1.0)
    }

    /// The widget: everything drawn, moved to sit in a margin.
    pub fn finish(mut self, id: &str) -> Widget {
        if self.min[0] > self.max[0] && self.dimensions.is_empty() {
            self.min = [0.0; 2];
            self.max = [1.0; 2];
        }
        self.clear_labels();
        let span = [self.max[0] - self.min[0], self.max[1] - self.min[1]];
        let margin = (span[0].max(span[1]) * 0.12).max(0.5);
        let shift = [margin - self.min[0], margin - self.min[1]];
        let mv = |p: &mut [f32; 2]| {
            p[0] += shift[0];
            p[1] += shift[1];
        };
        for shape in &mut self.shapes {
            match shape {
                DiagramShape::Path { points, .. } => points.iter_mut().for_each(mv),
                DiagramShape::Circle { center, .. } => mv(center),
                DiagramShape::Text { at, .. } => mv(at),
            }
        }
        for d in &mut self.dimensions {
            mv(&mut d.from);
            mv(&mut d.to);
        }
        for c in &mut self.callouts {
            mv(&mut c.anchor);
            mv(&mut c.at);
        }
        Widget::Diagram {
            id: id.into(),
            width: span[0] + 2.0 * margin,
            height: span[1] + 2.0 * margin,
            shapes: self.shapes,
            dimensions: self.dimensions,
            callouts: self.callouts,
        }
    }
}

/// The host draws a measure's text at a fixed size on a pill centred on
/// its line, whatever the drawing's scale: about this wide a character
/// and this tall, in pixels.
const LABEL_CHAR_PX: f32 = 6.6;
const LABEL_PAD_PX: f32 = 10.0;
const LABEL_HEIGHT_PX: f32 = 17.0;
/// The gap kept between a label and the point its line stands off from.
const LABEL_GAP_PX: f32 = 4.0;
/// The narrowest panel the drawing is fitted to and the tallest it is
/// drawn, in pixels: a scale guessed small keeps labels clear on any
/// wider panel too.
const PANEL_PX: f32 = 220.0;
const PANEL_HEIGHT_PX: f32 = 220.0;

impl Sketch<'_> {
    /// Half a measure's label across its line and along it, in pixels.
    fn label_half(d: &Dimension) -> ([f32; 2], [f32; 2]) {
        let w = d.text.chars().count() as f32 * LABEL_CHAR_PX + LABEL_PAD_PX;
        let (dx, dy) = (d.to[0] - d.from[0], d.to[1] - d.from[1]);
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let n = [-dy / len, dx / len];
        // The pill is upright: its half extent across the line and along it.
        let across = (w / 2.0 * n[0]).abs() + (LABEL_HEIGHT_PX / 2.0 * n[1]).abs();
        let along = (w / 2.0 * n[1]).abs() + (LABEL_HEIGHT_PX / 2.0 * n[0]).abs();
        ([w / 2.0, LABEL_HEIGHT_PX / 2.0], [across, along])
    }

    /// Where a measure's label sits, its line stood `offset` off.
    fn label_centre(d: &Dimension, offset: f32) -> [f32; 2] {
        let (dx, dy) = (d.to[0] - d.from[0], d.to[1] - d.from[1]);
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        [
            (d.from[0] + d.to[0]) / 2.0 - dy / len * offset,
            (d.from[1] + d.to[1]) / 2.0 + dx / len * offset,
        ]
    }

    /// Whether a measure's label, its line stood `offset` off, would sit
    /// on an edge of the drawing, kept `gap` clear.
    fn label_hits_edge(&self, d: &Dimension, offset: f32, half: [f32; 2], gap: f32) -> bool {
        self.box_hits_edge(Self::label_centre(d, offset), half, gap)
    }

    /// Whether a label centred at `c`, `half` across each way, would sit
    /// on an edge of the drawing, kept `gap` clear.
    fn box_hits_edge(&self, c: [f32; 2], half: [f32; 2], gap: f32) -> bool {
        let lo = [c[0] - half[0] - gap, c[1] - half[1] - gap];
        let hi = [c[0] + half[0] + gap, c[1] + half[1] + gap];
        let inside = |p: [f32; 2]| p[0] > lo[0] && p[0] < hi[0] && p[1] > lo[1] && p[1] < hi[1];
        let crosses = |a: [f32; 2], b: [f32; 2]| {
            (0..=16).any(|k| {
                let t = k as f32 / 16.0;
                inside([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t])
            })
        };
        self.shapes.iter().any(|shape| match shape {
            DiagramShape::Path {
                points,
                closed,
                stroke,
                ..
            } => {
                if *stroke == DiagramStroke::Axis {
                    return false;
                }
                let n = points.len();
                let segs = if *closed { n } else { n.saturating_sub(1) };
                (0..segs).any(|k| crosses(points[k], points[(k + 1) % n]))
            }
            DiagramShape::Circle { center, radius, .. } => {
                // The nearest point of the box to the centre against the
                // ring: on it when the box reaches the ring but does not
                // lie wholly inside it.
                let near = [center[0].clamp(lo[0], hi[0]), center[1].clamp(lo[1], hi[1])];
                let dn = ((near[0] - center[0]).powi(2) + (near[1] - center[1]).powi(2)).sqrt();
                let far = [
                    (center[0] - lo[0]).abs().max((hi[0] - center[0]).abs()),
                    (center[1] - lo[1]).abs().max((hi[1] - center[1]).abs()),
                ];
                let df = (far[0] * far[0] + far[1] * far[1]).sqrt();
                dn < *radius && df > *radius
            }
            DiagramShape::Text { .. } => false,
        })
    }

    /// Stand each measure off far enough that its label clears the part
    /// and the labels before it at the scale the panel draws at, slide
    /// each note out along its leader the same way, and take every label
    /// into the drawing's extent so none is cut off at the edge.
    ///
    /// The labels' size in millimetres hangs on the scale, and the scale
    /// on the extent the labels make, so the two are settled together:
    /// each round places every label afresh from where it was drawn.
    fn clear_labels(&mut self) {
        let base = (self.min, self.max);
        let wanted: Vec<f32> = self.dimensions.iter().map(|d| d.offset).collect();
        let wanted_at: Vec<[f32; 2]> = self.callouts.iter().map(|c| c.at).collect();
        let chains = self.chains();
        let mut scale = self.fit(base).0;
        for _ in 0..12 {
            self.place_labels(scale, &wanted, &wanted_at, &chains);
            let next = self.fit(base).0;
            let settled = (next - scale).abs() < scale * 0.01;
            scale = next;
            if settled {
                break;
            }
        }
        let (_, lo, hi) = self.fit(base);
        self.min = lo;
        self.max = hi;
    }

    /// For each measure, the first measure on its line: a chain is moved
    /// together so it stays a chain.
    fn chains(&self) -> Vec<usize> {
        let line_of = |d: &Dimension| {
            let (dx, dy) = (d.to[0] - d.from[0], d.to[1] - d.from[1]);
            let len = (dx * dx + dy * dy).sqrt().max(1e-6);
            let (ux, uy) = if dx < -1e-6 || (dx.abs() <= 1e-6 && dy < 0.0) {
                (-dx / len, -dy / len)
            } else {
                (dx / len, dy / len)
            };
            let p = Self::label_centre(d, d.offset);
            (ux, uy, p[0] * -uy + p[1] * ux)
        };
        let lines: Vec<(f32, f32, f32)> = self.dimensions.iter().map(line_of).collect();
        (0..lines.len())
            .map(|i| {
                (0..=i)
                    .find(|&j| {
                        let (a, b) = (lines[i], lines[j]);
                        (a.0 - b.0).abs() < 1e-3
                            && (a.1 - b.1).abs() < 1e-3
                            && (a.2 - b.2).abs() < 1e-3
                    })
                    .unwrap_or(i)
            })
            .collect()
    }

    /// The labels' boxes: centre and half size in pixels, the ends of the
    /// measures' lines as points.
    fn label_boxes(&self) -> Vec<([f32; 2], [f32; 2])> {
        let mut boxes = Vec::new();
        for d in &self.dimensions {
            let (dx, dy) = (d.to[0] - d.from[0], d.to[1] - d.from[1]);
            let len = (dx * dx + dy * dy).sqrt().max(1e-6);
            let n = [-dy / len * d.offset, dx / len * d.offset];
            for p in [d.from, d.to] {
                boxes.push(([p[0] + n[0], p[1] + n[1]], [0.0, 0.0]));
            }
            boxes.push((Self::label_centre(d, d.offset), Self::label_half(d).0));
        }
        for c in &self.callouts {
            boxes.push((c.at, Self::note_half(c)));
        }
        boxes
    }

    /// The scale the host draws at, with the extent it fits: what was
    /// drawn and the labels at their pixel size at that scale.
    fn fit(&self, base: ([f32; 2], [f32; 2])) -> (f32, [f32; 2], [f32; 2]) {
        let boxes = self.label_boxes();
        let extent = |scale: f32| {
            let (mut lo, mut hi) = base;
            for (c, half) in &boxes {
                let h = [half[0] / scale, half[1] / scale];
                lo = [lo[0].min(c[0] - h[0]), lo[1].min(c[1] - h[1])];
                hi = [hi[0].max(c[0] + h[0]), hi[1].max(c[1] + h[1])];
            }
            (lo, hi)
        };
        let scale_of = |(lo, hi): ([f32; 2], [f32; 2])| {
            let span = [(hi[0] - lo[0]).max(1e-3), (hi[1] - lo[1]).max(1e-3)];
            let margin = (span[0].max(span[1]) * 0.12).max(0.5) * 2.0;
            (PANEL_PX / (span[0] + margin)).min(PANEL_HEIGHT_PX / (span[1] + margin))
        };
        let mut scale = scale_of(extent(f32::MAX));
        for _ in 0..8 {
            scale = scale_of(extent(scale));
        }
        let (lo, hi) = extent(scale);
        (scale, lo, hi)
    }

    /// Half a note's label each way, in pixels.
    fn note_half(c: &Callout) -> [f32; 2] {
        let w = c.text.chars().count() as f32 * LABEL_CHAR_PX + LABEL_PAD_PX;
        [w / 2.0, LABEL_HEIGHT_PX / 2.0]
    }

    /// Place every label from where it was drawn, at `scale`: the notes
    /// first, then the measures in the order drawn, each clear of the
    /// edges and of the labels placed before it.
    fn place_labels(
        &mut self,
        scale: f32,
        wanted: &[f32],
        wanted_at: &[[f32; 2]],
        chains: &[usize],
    ) {
        let gap = LABEL_GAP_PX / 2.0 / scale;
        let step = 2.0 / scale;
        let mut placed: Vec<([f32; 2], [f32; 2])> = Vec::new();
        let clear_of = |placed: &[([f32; 2], [f32; 2])], c: [f32; 2], half: [f32; 2]| {
            !placed.iter().any(|(p, h)| {
                (c[0] - p[0]).abs() < half[0] + h[0] + gap
                    && (c[1] - p[1]).abs() < half[1] + h[1] + gap
            })
        };
        for (i, &to) in wanted_at.iter().enumerate() {
            let h = Self::note_half(&self.callouts[i]);
            let half = [h[0] / scale, h[1] / scale];
            let a = self.callouts[i].anchor;
            let run = ((to[0] - a[0]).powi(2) + (to[1] - a[1]).powi(2)).sqrt();
            let along = |t: f32| [a[0] + (to[0] - a[0]) * t, a[1] + (to[1] - a[1]) * t];
            let mut t = 1.0;
            if run > 1e-6 {
                for _ in 0..80 {
                    let at = along(t);
                    if clear_of(&placed, at, half) && !self.box_hits_edge(at, half, gap) {
                        break;
                    }
                    t += step / run;
                }
            }
            self.callouts[i].at = along(t);
            placed.push((along(t), half));
        }
        let mut need = vec![0.0f32; self.dimensions.len()];
        let mut owner = chains.to_vec();
        for i in 0..self.dimensions.len() {
            let d = &self.dimensions[i];
            let (h, [across, along]) = Self::label_half(d);
            let half = [h[0] / scale, h[1] / scale];
            let sign = if wanted[i] < 0.0 { -1.0 } else { 1.0 };
            // A chain whose labels would cover each other is dimensioned
            // apart instead.
            let mut g = chains[i];
            let covers = (0..i).any(|j| {
                let o = &self.dimensions[j];
                owner[j] == g && {
                    let (p, q) = (
                        Self::label_centre(d, wanted[i]),
                        Self::label_centre(o, wanted[i]),
                    );
                    let apart = ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt() * scale;
                    apart < along + Self::label_half(o).1[1] + LABEL_GAP_PX
                }
            });
            if covers {
                g = i;
            }
            let mut o = wanted[i]
                .abs()
                .max((across + LABEL_GAP_PX) / scale)
                .max(need[g]);
            for _ in 0..80 {
                let c = Self::label_centre(d, sign * o);
                if clear_of(&placed, c, half) && !self.label_hits_edge(d, sign * o, half, gap) {
                    break;
                }
                o += step;
            }
            need[g] = need[g].max(o);
            owner[i] = g;
            placed.push((Self::label_centre(d, sign * o), half));
        }
        for i in 0..self.dimensions.len() {
            let sign = if wanted[i] < 0.0 { -1.0 } else { 1.0 };
            self.dimensions[i].offset = sign * need[owner[i]];
        }
    }
}

/// The points of a shank drawn from `y_top` down to `y_bottom`, `r`
/// each side of `x`, broken in the middle with a jog when it is long
/// beside its width, so a long part keeps its head readable. Returns the
/// outline (closed) and the length it is drawn at.
pub fn shank(x: f64, r: f64, y_top: f64, length: f64) -> (Vec<[f64; 2]>, f64, bool) {
    let drawn = length.min((6.0 * r).max(length.min(8.0)));
    let broken = drawn < length - 1e-9;
    let y_bottom = y_top - drawn;
    if !broken {
        return (
            vec![
                [x - r, y_top],
                [x + r, y_top],
                [x + r, y_bottom],
                [x - r, y_bottom],
            ],
            drawn,
            false,
        );
    }
    // A break: the shank stops with a jog, leaves a gap, and goes on.
    let gap = (0.12 * drawn).max(0.3);
    let mid = y_top - drawn * 0.55;
    let (a, b) = (mid + gap / 2.0, mid - gap / 2.0);
    let jog = r * 0.5;
    let outline = vec![
        [x - r, y_top],
        [x + r, y_top],
        [x + r, a],
        [x + r - jog, a - gap * 0.25],
        [x + r, a - gap * 0.5],
        [x + r, b],
        [x + r, y_bottom],
        [x - r, y_bottom],
        [x - r, b],
        [x - r + jog, b + gap * 0.25],
        [x - r, b + gap * 0.5],
        [x - r, a],
    ];
    (outline, drawn, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sketch_fits_a_margin_around_what_it_drew() {
        let mut s = Sketch::new(Some("length"));
        s.rect([-2.0, -10.0], [2.0, 0.0]);
        s.height("length", 2.0, 0.0, -10.0, -3.0, "L 10");
        s.height("other", -2.0, 0.0, -10.0, 3.0, "x");
        let Widget::Diagram {
            width,
            height,
            dimensions,
            shapes,
            ..
        } = s.finish("d")
        else {
            panic!("a diagram");
        };
        assert!(width > 4.0 && height > 10.0);
        assert_eq!(dimensions.len(), 2);
        assert!(dimensions[0].emphasis && !dimensions[1].emphasis);
        // Everything sits inside the drawing.
        for shape in &shapes {
            if let DiagramShape::Path { points, .. } = shape {
                for p in points {
                    assert!(p[0] >= 0.0 && p[1] >= 0.0 && p[0] <= width && p[1] <= height);
                }
            }
        }
    }

    #[test]
    fn a_long_shank_is_drawn_broken() {
        let (short, drawn, broken) = shank(0.0, 1.5, 0.0, 8.0);
        assert!(!broken && short.len() == 4 && (drawn - 8.0).abs() < 1e-9);
        let (long, drawn, broken) = shank(0.0, 1.5, 0.0, 60.0);
        assert!(broken && long.len() == 12 && drawn < 60.0);
    }
}
