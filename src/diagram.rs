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
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let n = [-dy / len * offset as f32, dx / len * offset as f32];
        let edge = 1.3;
        self.cover([f[0] + n[0] * edge, f[1] + n[1] * edge]);
        self.cover([t[0] + n[0] * edge, t[1] + n[1] * edge]);
        self.dimensions.push(Dimension {
            from: f,
            to: t,
            offset: offset as f32,
            text: text.into(),
            emphasis: self.focus == Some(key),
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
        });
        self
    }

    /// The standoff for a drawing of a part about `extent` across.
    pub fn standoff(extent: f64) -> f64 {
        (extent * STANDOFF).max(1.0)
    }

    /// The widget: everything drawn, moved to sit in a margin.
    pub fn finish(mut self, id: &str) -> Widget {
        if self.min[0] > self.max[0] {
            self.min = [0.0; 2];
            self.max = [1.0; 2];
        }
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
