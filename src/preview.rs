//! Every tool's drawing laid out the way the host draws it (a 220 px
//! panel, at most 220 px tall, 11 px mono labels on pills), checked for
//! labels that cover each other or run off the edge. With
//! `PREVIEW=/path/page.html` (and `PREVIEW_ONLY=screw,nut` to narrow it)
//! the drawings are also written to a page to look at.

use crate::parts::{Ctx, Defaults};
use printcad_bench_sdk::api::{DiagramShape, DiagramStroke, Widget};

const PANEL: f32 = 220.0;
const MAX_H: f32 = 220.0;
const CHAR: f32 = 6.6;
const PILL_H: f32 = 17.0;

#[derive(Clone, Copy)]
struct R {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

fn pill(c: (f32, f32), text: &str) -> R {
    let w = text.chars().count() as f32 * CHAR + 10.0;
    R {
        x0: c.0 - w / 2.0,
        y0: c.1 - PILL_H / 2.0,
        x1: c.0 + w / 2.0,
        y1: c.1 + PILL_H / 2.0,
    }
}

fn hit(a: R, b: R) -> bool {
    a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1
}

fn seg_hits(r: R, p: (f32, f32), q: (f32, f32)) -> bool {
    (0..=20).any(|i| {
        let t = i as f32 / 20.0;
        let x = p.0 + (q.0 - p.0) * t;
        let y = p.1 + (q.1 - p.1) * t;
        x > r.x0 + 1.0 && x < r.x1 - 1.0 && y > r.y0 + 1.0 && y < r.y1 - 1.0
    })
}

fn render(widget: &Widget) -> (String, usize, usize) {
    let Widget::Diagram {
        width,
        height,
        shapes,
        dimensions,
        callouts,
        ..
    } = widget
    else {
        return (String::new(), 0, 0);
    };
    let scale = (PANEL / width).min(MAX_H / height);
    let (w, h) = (width * scale, height * scale);
    let ox = (PANEL - w) / 2.0;
    let at = |p: [f32; 2]| (ox + p[0] * scale, h - p[1] * scale);
    let mut svg =
        format!("<svg width='{PANEL}' height='{h}' style='background:#1e1e22;margin:4px'>");
    let mut lines: Vec<((f32, f32), (f32, f32))> = Vec::new();
    for s in shapes {
        match s {
            DiagramShape::Path {
                points,
                closed,
                stroke,
                fill,
            } => {
                let mut pts: Vec<(f32, f32)> = points.iter().map(|p| at(*p)).collect();
                if *closed && !pts.is_empty() {
                    pts.push(pts[0]);
                }
                let (col, dash) = match stroke {
                    DiagramStroke::Outline => ("#ddd", ""),
                    DiagramStroke::Hidden => ("#aaa", "4 3"),
                    DiagramStroke::Axis => ("#777", "10 4"),
                    _ => ("#777", ""),
                };
                let d: Vec<String> = pts
                    .iter()
                    .map(|p| format!("{:.1},{:.1}", p.0, p.1))
                    .collect();
                svg += &format!(
                    "<polyline points='{}' stroke='{col}' stroke-dasharray='{dash}' fill='{}'/>",
                    d.join(" "),
                    if *fill { "#333338" } else { "none" }
                );
                if matches!(stroke, DiagramStroke::Outline) {
                    lines.extend(pts.windows(2).map(|w| (w[0], w[1])));
                }
            }
            DiagramShape::Circle {
                center,
                radius,
                stroke: _,
                fill,
            } => {
                let c = at(*center);
                svg += &format!(
                    "<circle cx='{}' cy='{}' r='{}' stroke='#ddd' fill='{}'/>",
                    c.0,
                    c.1,
                    radius * scale,
                    if *fill { "#333338" } else { "none" }
                );
            }
            DiagramShape::Text { at: p, text, .. } => {
                let c = at(*p);
                svg += &format!(
                    "<text x='{}' y='{}' fill='#aaa' font-size='11' text-anchor='middle'>{text}</text>",
                    c.0, c.1
                );
            }
        }
    }
    let mut pills: Vec<(R, String, bool)> = Vec::new();
    for d in dimensions {
        let (f, t) = (at(d.from), at(d.to));
        let (dx, dy) = (t.0 - f.0, t.1 - f.1);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.5 {
            continue;
        }
        let dir = (dx / len, dy / len);
        let n = (dir.1, -dir.0);
        let o = (n.0 * d.offset * scale, n.1 * d.offset * scale);
        let (p1, p2) = ((f.0 + o.0, f.1 + o.1), (t.0 + o.0, t.1 + o.1));
        let s = d.offset.signum() * 4.0;
        for (a, b) in [
            (f, (p1.0 + n.0 * s, p1.1 + n.1 * s)),
            (t, (p2.0 + n.0 * s, p2.1 + n.1 * s)),
        ] {
            svg += &format!(
                "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#666'/>",
                a.0, a.1, b.0, b.1
            );
            lines.push((a, b));
        }
        svg += &format!(
            "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#aaa'/>",
            p1.0, p1.1, p2.0, p2.1
        );
        let c = ((p1.0 + p2.0) / 2.0, (p1.1 + p2.1) / 2.0);
        pills.push((pill(c, &d.text), d.text.clone(), d.emphasis));
    }
    for c in callouts {
        let (a, l) = (at(c.anchor), at(c.at));
        svg += &format!(
            "<line x1='{}' y1='{}' x2='{}' y2='{}' stroke='#aaa'/><circle cx='{}' cy='{}' r='2.5' fill='#aaa'/>",
            a.0, a.1, l.0, l.1, a.0, a.1
        );
        pills.push((pill(l, &c.text), c.text.clone(), c.emphasis));
    }
    let mut clash = 0;
    for i in 0..pills.len() {
        for j in i + 1..pills.len() {
            if hit(pills[i].0, pills[j].0) {
                println!("   {:?} covers {:?}", pills[i].1, pills[j].1);
                clash += 1;
            }
        }
    }
    let over = pills
        .iter()
        .filter(|p| lines.iter().any(|l| seg_hits(p.0, l.0, l.1)))
        .count();
    let clipped = pills
        .iter()
        .filter(|p| p.0.x0 < 0.0 || p.0.x1 > PANEL || p.0.y0 < 0.0 || p.0.y1 > h)
        .inspect(|p| println!("   {:?} runs off the edge", p.1))
        .count();
    for (r, text, _) in &pills {
        svg += &format!(
            "<rect x='{}' y='{}' width='{}' height='{}' rx='3' fill='#2a2a30'/><text x='{}' y='{}' fill='#eee' font-family='monospace' font-size='11' text-anchor='middle'>{}</text>",
            r.x0,
            r.y0,
            r.x1 - r.x0,
            r.y1 - r.y0,
            (r.x0 + r.x1) / 2.0,
            r.y1 - 5.0,
            text
        );
    }
    svg += "</svg>";
    (svg, clash + clipped, over)
}

#[test]
fn no_drawing_has_labels_on_each_other_or_off_its_edge() {
    let out = std::env::var("PREVIEW").ok();
    let mut page =
        String::from("<html><body style='background:#111;color:#ccc;font-family:sans-serif'>");
    let mut total = (0, 0);
    for size in ["M3", "M8"] {
        let defaults = Defaults {
            size: size.into(),
            ..Defaults::default()
        };
        let only = std::env::var("PREVIEW_ONLY").unwrap_or_default();
        for tool in crate::TOOLS
            .iter()
            .filter(|t| only.is_empty() || only.split(',').any(|o| t.suffix.contains(o)))
        {
            let part = (tool.seed)(&defaults);
            let ctx = Ctx {
                feature: "f",
                focus: None,
            };
            let (mut svg, mut clash, mut over) = (String::new(), 0, 0);
            for w in part
                .panel(&ctx)
                .iter()
                .filter(|w| matches!(w, Widget::Diagram { .. }))
            {
                let (a, b, c) = render(w);
                svg += &a;
                svg += "<br>";
                clash += b;
                over += c;
            }
            total.0 += clash;
            total.1 += over;
            println!("{size} {:<28} clash {clash} over-lines {over}", tool.suffix);
            page += &format!(
                "<div style='display:inline-block;vertical-align:top'><div>{size} {} — clash {clash}, on lines {over}</div>{svg}</div>",
                tool.suffix
            );
        }
    }
    println!("TOTAL clash {} over-lines {}", total.0, total.1);
    page += "</body></html>";
    if let Some(out) = out {
        std::fs::write(out, page).unwrap();
    }
    assert_eq!(total.0, 0, "labels covering each other or cut off");
}
