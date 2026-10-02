//! Screws: socket head cap, button head, countersunk, hex head, low head
//! and set screws, sized by their standards.
//!
//! A screw stands on its axis along Z with the underside of its head on
//! `z = 0`, the head above and the shank below; a set screw's top is on
//! `z = 0`. A countersunk screw's length is over all, as its standard
//! measures it; every other length is from under the head.

use printcad_bench_sdk::Value;
use printcad_bench_sdk::api::kernel_api::{BooleanOp, ProfileSegment, SolidOp};
use printcad_bench_sdk::api::{DiagramStroke, PanelEvent, Parameter, Widget};
use serde::{Deserialize, Serialize};

use super::{
    Ctx, Defaults, Family, Part, arg_bool, arg_f64, arg_str, choice, default_length, fmt, group,
    index_of, length, note, number, toggle,
};
use crate::diagram::{Sketch, shank};
use crate::geom::{self, across_corners, arc, polygon, revolve, revolve_segments};
use crate::standards::{self, ScrewTable};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
// `LowHead` is the head's name; the lint would have it lose it.
#[allow(clippy::enum_variant_names)]
pub enum Head {
    SocketCap,
    Button,
    Countersunk,
    Hex,
    LowHead,
    Set,
    /// A knurled disc on a shoulder, turned by hand.
    Thumb,
}

impl Head {
    pub const ALL: [Head; 7] = [
        Head::SocketCap,
        Head::Button,
        Head::Countersunk,
        Head::Hex,
        Head::LowHead,
        Head::Set,
        Head::Thumb,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Head::SocketCap => "Socket head cap",
            Head::Button => "Button head",
            Head::Countersunk => "Countersunk",
            Head::Hex => "Hex head",
            Head::LowHead => "Low head cap",
            Head::Set => "Set screw",
            Head::Thumb => "Knurled thumb",
        }
    }

    /// The short name a part is labelled with.
    fn short(self) -> &'static str {
        match self {
            Head::SocketCap => "cap screw",
            Head::Button => "button head screw",
            Head::Countersunk => "countersunk screw",
            Head::Hex => "hex bolt",
            Head::LowHead => "low head screw",
            Head::Set => "set screw",
            Head::Thumb => "thumb screw",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Head::SocketCap => "screw-socket",
            Head::Button => "screw-button",
            Head::Countersunk => "screw-countersunk",
            Head::Hex => "screw-hex",
            Head::LowHead => "screw-low-head",
            Head::Set => "screw-set",
            Head::Thumb => "screw-thumb",
        }
    }

    /// The tool that makes one, by its id's last part.
    pub fn tool(self) -> &'static str {
        match self {
            Head::SocketCap => "socket_cap",
            Head::Button => "button",
            Head::Countersunk => "countersunk",
            Head::Hex => "hex_bolt",
            Head::LowHead => "low_head",
            Head::Set => "set_screw",
            Head::Thumb => "thumb",
        }
    }

    pub fn named(name: &str) -> Option<Head> {
        Head::ALL.into_iter().find(|h| {
            h.tool().eq_ignore_ascii_case(name)
                || h.name().eq_ignore_ascii_case(name)
                || h.short().eq_ignore_ascii_case(name)
        })
    }

    /// The standards that size this head, the first the default.
    pub fn standards(self) -> &'static [ScrewTable] {
        match self {
            Head::SocketCap => &[standards::ISO_4762, standards::ASME_B18_3],
            Head::Button => &[standards::ISO_7380],
            Head::Countersunk => &[standards::ISO_10642],
            Head::Hex => &[standards::ISO_4017],
            Head::LowHead => &[standards::DIN_7984],
            Head::Set => &[standards::ISO_4026],
            Head::Thumb => &[standards::DIN_464],
        }
    }

    fn has_socket(self) -> bool {
        !matches!(self, Head::Hex | Head::Thumb)
    }

    /// The head a size off the tables gets, in proportion to its thread:
    /// `(dk, k, s, t)` as the standards run.
    fn proportions(self, d: f64) -> (f64, f64, f64, f64) {
        let r = |v: f64| (v * 10.0).round() / 10.0;
        match self {
            Head::SocketCap => (r(1.5 * d), r(d), r(0.8 * d), r(0.5 * d)),
            Head::LowHead => (r(1.5 * d), r(0.6 * d), r(0.7 * d), r(0.4 * d)),
            Head::Button => (r(1.9 * d), r(0.55 * d), r(0.55 * d), r(0.3 * d)),
            Head::Countersunk => (r(2.0 * d), r(0.6 * d), r(0.6 * d), r(0.35 * d)),
            Head::Hex => (r(1.6 * d + 0.5), r(0.65 * d), 0.0, 0.0),
            Head::Set => (r(0.6 * d), 0.0, r(0.5 * d), r(0.5 * d)),
            Head::Thumb => (r(4.0 * d), r(2.4 * d), r(2.0 * d), r(0.8 * d)),
        }
    }
}

/// A screw's data. Lengths in millimetres.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Screw {
    pub head: Head,
    pub standard: String,
    pub size: String,
    pub length: f64,
    /// The thread's diameter and pitch.
    pub d: f64,
    pub pitch: f64,
    /// The head's diameter; a hex head's width across flats; a set
    /// screw's point diameter.
    pub dk: f64,
    /// The head's height.
    pub k: f64,
    /// The drive's hex key, across flats.
    pub s: f64,
    /// The drive's depth.
    pub t: f64,
    /// How far up from the tip the thread runs; 0 for the whole shank.
    #[serde(default)]
    pub thread_length: f64,
    /// The head's dimensions are the user's, not the table's.
    #[serde(default)]
    pub custom: bool,
    /// Model the drive recess.
    #[serde(default = "yes")]
    pub socket: bool,
    /// Cut the thread into the shank.
    #[serde(default)]
    pub thread: bool,
}

fn yes() -> bool {
    true
}

impl Screw {
    /// A screw of `head` sized `size` by the head's first standard.
    pub fn new(head: Head, size: &str, defaults: &Defaults) -> Screw {
        let mut screw = Screw {
            head,
            standard: head.standards()[0].name.into(),
            size: size.into(),
            length: 0.0,
            d: 0.0,
            pitch: 0.0,
            dk: 0.0,
            k: 0.0,
            s: 0.0,
            t: 0.0,
            thread_length: 0.0,
            custom: false,
            socket: defaults.socket,
            thread: defaults.thread,
        };
        screw.refill();
        screw.length = default_length(screw.d);
        screw.thread_length = screw.standard_thread_length();
        screw
    }

    pub fn table(&self) -> &'static ScrewTable {
        self.head
            .standards()
            .iter()
            .find(|t| t.name == self.standard)
            .unwrap_or(&self.head.standards()[0])
    }

    fn sizes(&self) -> Vec<&'static str> {
        self.table().rows.iter().map(|r| r.size).collect()
    }

    /// Take the dimensions of `size` from the standard; a size the
    /// standard lacks takes the nearest it has.
    pub fn refill(&mut self) {
        let table = self.table();
        self.standard = table.name.into();
        let row = table
            .rows
            .iter()
            .find(|r| r.size.eq_ignore_ascii_case(&self.size))
            .or_else(|| {
                let want = standards::metric(&self.size)
                    .map(|(d, _)| d)
                    .unwrap_or(self.d);
                table
                    .rows
                    .iter()
                    .min_by(|a, b| (a.d - want).abs().total_cmp(&(b.d - want).abs()))
            })
            .copied()
            .unwrap_or(table.rows[0]);
        self.size = row.size.into();
        self.d = row.d;
        self.pitch = row.pitch;
        self.dk = row.dk;
        self.k = row.k;
        self.s = row.s;
        self.t = row.t;
    }

    /// The thread length the standard gives a screw of this size: twice
    /// the diameter and twelve, the socket head standards' reference
    /// length for screws up to 125 long; a set screw and a hex bolt are
    /// threaded to the head.
    fn standard_thread_length(&self) -> f64 {
        match self.head {
            Head::Set | Head::Hex => 0.0,
            Head::Thumb => 3.0 * self.d,
            _ => 2.0 * self.d + 12.0,
        }
    }

    /// Size the screw `size`, a diameter off the tables, in proportion
    /// to its thread.
    fn proportion(&mut self, size: &str, d: f64, pitch: f64) {
        self.size = size.trim().to_uppercase();
        self.d = d;
        self.pitch = pitch;
        (self.dk, self.k, self.s, self.t) = self.head.proportions(d);
        self.length = default_length(d);
        self.thread_length = self.standard_thread_length();
    }

    /// How many facets a knurled head shows round.
    fn knurl_facets(&self) -> u32 {
        ((std::f64::consts::PI * self.dk / 1.2).round() as u32).clamp(24, 60)
    }

    /// The shank's length: from under the head, or a countersunk head's
    /// top, to the tip.
    fn shank_length(&self) -> f64 {
        match self.head {
            Head::Countersunk => (self.length - self.k).max(0.0),
            _ => self.length,
        }
    }

    /// The thread's run up from the tip as built.
    fn threaded(&self) -> f64 {
        let shank = self.shank_length();
        if self.thread_length <= 0.0 {
            shank
        } else {
            self.thread_length.min(shank)
        }
    }

    /// A button head's flat top: round the socket, a quarter of the head
    /// at least.
    fn flat_top(&self) -> f64 {
        (across_corners(self.s) / 2.0 + 0.2)
            .max(0.25 * self.dk)
            .min(0.45 * self.dk)
    }

    /// A button head's dome: the centre (on the axis) and radius of the
    /// circle through the rim and the edge of the flat top.
    fn dome(&self) -> (f64, f64) {
        let (rk, k, rf) = (self.dk / 2.0, self.k, self.flat_top());
        let zc = (k * k + rf * rf - rk * rk) / (2.0 * k);
        (zc, (rk * rk + zc * zc).sqrt())
    }

    /// The chamfer at the tip.
    fn tip_chamfer(&self) -> f64 {
        (0.6 * self.pitch).min(self.d / 4.0)
    }

    /// The half-section of a cylindrical head and the shank, in `(r, z)`.
    fn capped_section(&self, head_chamfer: f64) -> Vec<[f64; 2]> {
        let (r, rk, k, l, c) = (
            self.d / 2.0,
            self.dk / 2.0,
            self.k,
            self.length,
            self.tip_chamfer(),
        );
        vec![
            [0.0, -l],
            [r - c, -l],
            [r, -l + c],
            [r, 0.0],
            [rk, 0.0],
            [rk, k - head_chamfer],
            [rk - head_chamfer, k],
            [0.0, k],
        ]
    }

    /// What goes on after the thread is cut: a hex bolt's head, fused on
    /// and crowned by an envelope tall enough to leave the shank alone.
    fn head_ops(&self) -> Vec<SolidOp> {
        match self.head {
            Head::Hex => {
                let e = across_corners(self.dk);
                let crown = (0.08 * self.dk).min(0.3 * self.k);
                vec![
                    geom::prism(
                        &geom::hexagon(self.dk),
                        Vec::new(),
                        0.0,
                        self.k,
                        BooleanOp::Fuse,
                    ),
                    geom::crown(e, -self.length - 1.0, self.k, crown, 30.0, false, true),
                ]
            }
            Head::Thumb => {
                // The knurled disc, faceted, fused on the shoulder's top
                // and a little down into it.
                let n = self.knurl_facets();
                let across = self.dk * (std::f64::consts::PI / f64::from(n)).cos();
                let z = self.k - self.t;
                vec![geom::prism(
                    &geom::regular(n, across),
                    Vec::new(),
                    z,
                    self.t,
                    BooleanOp::Fuse,
                )]
            }
            _ => Vec::new(),
        }
    }

    pub fn body_ops(&self) -> Vec<SolidOp> {
        let (r, l, c) = (self.d / 2.0, self.length, self.tip_chamfer());
        match self.head {
            Head::SocketCap | Head::LowHead => {
                vec![revolve(
                    &self.capped_section((0.08 * self.dk).min(0.3 * self.k)),
                    BooleanOp::NewSolid,
                )]
            }
            Head::Button => {
                // The dome: an arc from the head's rim up to a small flat
                // top, on a circle centred on the axis. The flat takes the
                // socket, so the recess is cut through a plane.
                let (rk, k) = (self.dk / 2.0, self.k);
                let rf = self.flat_top();
                let (zc, radius) = self.dome();
                let mut segments =
                    polygon(&[[0.0, -l], [r - c, -l], [r, -l + c], [r, 0.0], [rk, 0.0]]);
                segments.pop();
                segments.push(arc([0.0, zc], radius, [rk, 0.0], [rf, k]));
                segments.push(ProfileSegment::Line {
                    start: [rf, k],
                    end: [0.0, k],
                });
                segments.push(ProfileSegment::Line {
                    start: [0.0, k],
                    end: [0.0, -l],
                });
                vec![revolve_segments(segments, BooleanOp::NewSolid)]
            }
            Head::Countersunk => {
                let shank = self.shank_length();
                vec![revolve(
                    &[
                        [0.0, -shank],
                        [r - c, -shank],
                        [r, -shank + c],
                        [r, 0.0],
                        [self.dk / 2.0, self.k],
                        [0.0, self.k],
                    ],
                    BooleanOp::NewSolid,
                )]
            }
            Head::Hex => {
                // The shank alone, reaching up into the head; the head is
                // fused on after the thread (`head_ops`), so the groove is
                // cut into a plain body of revolution.
                vec![revolve(
                    &[
                        [0.0, -l],
                        [r - c, -l],
                        [r, -l + c],
                        [r, self.k / 2.0],
                        [0.0, self.k / 2.0],
                    ],
                    BooleanOp::NewSolid,
                )]
            }
            Head::Set => {
                let point = (self.d - self.dk) / 2.0;
                vec![revolve(
                    &[
                        [0.0, -l],
                        [self.dk / 2.0, -l],
                        [r, -l + point],
                        [r, 0.0],
                        [0.0, 0.0],
                    ],
                    BooleanOp::NewSolid,
                )]
            }
            Head::Thumb => {
                // The shank and the shoulder, reaching up into the disc
                // that is fused on after the thread.
                let rs = self.s / 2.0;
                let top = self.k - self.t + (self.t / 2.0).min(0.5);
                vec![revolve(
                    &[
                        [0.0, -l],
                        [r - c, -l],
                        [r, -l + c],
                        [r, 0.0],
                        [rs, 0.0],
                        [rs, top],
                        [0.0, top],
                    ],
                    BooleanOp::NewSolid,
                )]
            }
        }
    }
}

impl Part for Screw {
    const FAMILY: Family = Family::Screw;

    fn label(&self) -> String {
        format!("{} × {} {}", self.size, fmt(self.length), self.head.short())
    }

    fn icon(&self) -> &'static str {
        self.head.icon()
    }

    fn problem(&self) -> Option<String> {
        if self.d <= 0.0 || self.length <= 0.0 || self.pitch <= 0.0 {
            return Some("The diameter, the pitch and the length must be more than 0.".into());
        }
        if self.head == Head::Set {
            if self.dk >= self.d {
                return Some("The point must be narrower than the thread.".into());
            }
        } else {
            if self.dk <= self.d || self.k <= 0.0 {
                return Some("The head must be wider than the thread and taller than 0.".into());
            }
            if self.head == Head::Countersunk && self.length <= self.k {
                return Some("A countersunk screw must be longer than its head.".into());
            }
        }
        if self.head == Head::Thumb
            && (self.s <= self.d || self.s >= self.dk || self.t <= 0.0 || self.t >= self.k)
        {
            return Some(
                "The shoulder must lie between the thread and the knurl, and the knurl within the head."
                    .into(),
            );
        }
        if self.head.has_socket() && self.socket {
            if self.s <= 0.0 || self.t <= 0.0 {
                return Some("The socket needs a key size and a depth.".into());
            }
            let head = if self.head == Head::Set {
                self.length
            } else {
                self.k
            };
            if self.t >= head {
                return Some("The socket is deeper than the head.".into());
            }
            let across = if self.head == Head::Set {
                self.d
            } else {
                self.dk
            };
            if across_corners(self.s) >= across - 0.2 {
                return Some("The socket is wider than the head.".into());
            }
        }
        if self.thread && self.pitch * 1.3 >= self.d {
            return Some("The thread is too coarse for its diameter to be cut.".into());
        }
        None
    }

    fn axis(&self) -> ([f64; 3], [f64; 3]) {
        let top = if self.head == Head::Set { 0.0 } else { self.k };
        super::z_axis(-self.shank_length(), top)
    }

    fn ops(&self) -> Vec<SolidOp> {
        let mut ops = self.body_ops();
        // The thread first, while the solid is a plain body of revolution;
        // a hex head goes on after it, and the recess cuts the head alone.
        if self.thread {
            let shank = self.shank_length();
            let run = self.threaded();
            let lead_in = run < shank - 1e-6;
            ops.push(geom::external_thread(
                self.d,
                self.pitch,
                -(shank - run),
                run,
                lead_in,
            ));
        }
        ops.extend(self.head_ops());
        if self.head.has_socket() && self.socket {
            let top = if self.head == Head::Set { 0.0 } else { self.k };
            ops.push(geom::hex_socket(self.s, self.t, top));
        }
        ops
    }

    fn panel(&self, ctx: &Ctx) -> Vec<Widget> {
        let standards: Vec<&str> = self.head.standards().iter().map(|t| t.name).collect();
        let sizes = self.sizes();
        let heads: Vec<&str> = Head::ALL.iter().map(|h| h.name()).collect();
        let mut widgets = vec![
            self.drawing(ctx),
            choice(
                "head",
                "Head",
                &heads,
                Head::ALL.iter().position(|h| *h == self.head).unwrap_or(0),
            ),
            choice(
                "standard",
                "Standard",
                &standards,
                index_of(&standards, &self.standard),
            ),
            choice("size", "Size", &sizes, index_of(&sizes, &self.size)),
            number(ctx, "length", "Length", self.length, 0.1, 1),
        ];
        let mut dims = vec![toggle("custom", "Custom dimensions", self.custom)];
        if self.custom {
            dims.push(number(ctx, "d", "Thread diameter", self.d, 0.1, 2));
            dims.push(number(ctx, "pitch", "Pitch", self.pitch, 0.05, 2));
            match self.head {
                Head::Set => dims.push(number(ctx, "dk", "Point diameter", self.dk, 0.0, 2)),
                Head::Hex => {
                    dims.push(number(ctx, "dk", "Across flats", self.dk, 0.1, 2));
                    dims.push(number(ctx, "k", "Head height", self.k, 0.1, 2));
                }
                Head::Thumb => {
                    dims.push(number(ctx, "dk", "Knurl diameter", self.dk, 0.1, 2));
                    dims.push(number(ctx, "k", "Head height", self.k, 0.1, 2));
                    dims.push(number(ctx, "s", "Shoulder diameter", self.s, 0.1, 2));
                    dims.push(number(ctx, "t", "Knurl height", self.t, 0.1, 2));
                }
                _ => {
                    dims.push(number(ctx, "dk", "Head diameter", self.dk, 0.1, 2));
                    dims.push(number(ctx, "k", "Head height", self.k, 0.1, 2));
                }
            }
            if self.head.has_socket() {
                dims.push(number(ctx, "s", "Hex key", self.s, 0.1, 2));
                dims.push(number(ctx, "t", "Socket depth", self.t, 0.1, 2));
            }
        }
        widgets.push(group("Dimensions", self.custom, dims));
        let mut options = Vec::new();
        if self.head.has_socket() {
            options.push(toggle("socket", "Drive recess", self.socket));
        }
        options.push(toggle("thread", "Modelled thread", self.thread));
        if self.thread {
            options.push(number(
                ctx,
                "thread_length",
                "Thread length (0: all)",
                self.thread_length,
                0.0,
                1,
            ));
            options.push(super::text(
                "A modelled thread takes the kernel a while on a long screw.",
            ));
        }
        widgets.push(group("Options", true, options));
        widgets.push(group(
            "Holes for it",
            true,
            vec![super::text(super::holes_text(self.d, self.pitch))],
        ));
        widgets.extend(note(self.problem()));
        widgets
    }

    fn parameters(&self) -> Vec<Parameter> {
        vec![
            length("length", "Length"),
            length("d", "Thread diameter"),
            length("pitch", "Pitch"),
            length("dk", "Head diameter"),
            length("k", "Head height"),
            length("s", "Hex key"),
            length("t", "Socket depth"),
            length("thread_length", "Thread length"),
        ]
    }

    fn apply(&mut self, event: &PanelEvent) -> bool {
        match event {
            PanelEvent::Choice { id, index } => match id.as_str() {
                "head" => {
                    self.head = Head::ALL.get(*index).copied().unwrap_or(Head::SocketCap);
                    self.standard = self.head.standards()[0].name.into();
                    self.refill();
                    self.thread_length = self.standard_thread_length();
                }
                "standard" => {
                    self.standard = self.head.standards()
                        [*index.min(&(self.head.standards().len() - 1))]
                    .name
                    .into();
                    self.refill();
                }
                "size" => {
                    if let Some(size) = self.sizes().get(*index) {
                        self.size = (*size).into();
                        self.refill();
                        self.thread_length = self.standard_thread_length();
                    }
                }
                _ => return false,
            },
            PanelEvent::Number { id, value } => match id.as_str() {
                "length" => self.length = *value,
                "d" => self.d = *value,
                "pitch" => self.pitch = *value,
                "dk" => self.dk = *value,
                "k" => self.k = *value,
                "s" => self.s = *value,
                "t" => self.t = *value,
                "thread_length" => self.thread_length = *value,
                _ => return false,
            },
            PanelEvent::Toggle { id, on } => match id.as_str() {
                "custom" => {
                    self.custom = *on;
                    if !*on {
                        self.refill();
                    }
                }
                "socket" => self.socket = *on,
                "thread" => self.thread = *on,
                _ => return false,
            },
            _ => return false,
        }
        true
    }

    fn with_args(args: &Value, defaults: &Defaults) -> Result<Self, String> {
        let head = match arg_str(args, "head") {
            None => Head::SocketCap,
            Some(name) => Head::named(name).ok_or_else(|| {
                format!(
                    "`{name}` is not a head: {}",
                    Head::ALL.map(|h| h.tool()).join(", ")
                )
            })?,
        };
        let size = arg_str(args, "size").unwrap_or(&defaults.size);
        let mut screw = Screw::new(head, size, defaults);
        if let Some(standard) = arg_str(args, "standard") {
            let table = head
                .standards()
                .iter()
                .find(|t| t.name.eq_ignore_ascii_case(standard) || t.name.starts_with(standard))
                .ok_or_else(|| {
                    format!(
                        "`{standard}` does not size a {}",
                        head.name().to_lowercase()
                    )
                })?;
            screw.standard = table.name.into();
            screw.size = size.into();
            screw.refill();
            screw.length = default_length(screw.d);
            screw.thread_length = screw.standard_thread_length();
        }
        if !screw.size.eq_ignore_ascii_case(size) && arg_str(args, "size").is_some() {
            // A size the table lacks is made in proportion to its thread.
            let (d, pitch) = standards::metric_any(size)
                .ok_or_else(|| format!("{} has no size `{size}`", screw.standard))?;
            screw.proportion(size, d, pitch);
        }
        if arg_str(args, "size").is_none()
            && let Some(d) = arg_f64(args, "d")
            && (d - screw.d).abs() > 1e-9
        {
            // A diameter alone names its size, from the table or in
            // proportion.
            let name = standards::size_name(d);
            if screw.sizes().iter().any(|s| s.eq_ignore_ascii_case(&name)) {
                screw.size = name;
                screw.refill();
                screw.length = default_length(screw.d);
                screw.thread_length = screw.standard_thread_length();
            } else {
                screw.proportion(&name, d, standards::coarse_pitch(d));
            }
        }
        if let Some(length) = arg_f64(args, "length") {
            screw.length = length;
        }
        for (key, slot) in [
            ("d", &mut screw.d),
            ("pitch", &mut screw.pitch),
            ("dk", &mut screw.dk),
            ("k", &mut screw.k),
            ("s", &mut screw.s),
            ("t", &mut screw.t),
        ] {
            if let Some(v) = arg_f64(args, key) {
                *slot = v;
                screw.custom = true;
            }
        }
        if let Some(v) = arg_f64(args, "thread_length") {
            screw.thread_length = v;
        }
        if let Some(on) = arg_bool(args, "socket") {
            screw.socket = on;
        }
        if let Some(on) = arg_bool(args, "thread") {
            screw.thread = on;
        }
        Ok(screw)
    }
}

impl Screw {
    /// The side view: the head over the shank, the thread's size under
    /// the tip, the length beside it.
    fn drawing(&self, ctx: &Ctx) -> Widget {
        let mut s = Sketch::new(ctx.focus);
        let r = self.d / 2.0;
        let shank_len = self.shank_length();
        let (outline, drawn, broken) = shank(0.0, r, 0.0, shank_len);
        let bottom = -drawn;
        let wide = match self.head {
            Head::Set => self.d,
            Head::Hex => across_corners(self.dk),
            _ => self.dk,
        };
        let off = Sketch::standoff(wide.max(drawn + self.k) * 0.6);
        // The shank, and the thread on it.
        if self.head != Head::Set {
            s.poly(&outline, DiagramStroke::Outline, !broken);
        }
        let scale = if shank_len > 0.0 {
            drawn / shank_len
        } else {
            1.0
        };
        let run = self.threaded() * scale;
        if self.thread || (self.thread_length > 0.0 && self.thread_length < shank_len) {
            let top = bottom + run;
            s.line(&[[-r, top], [r, top]], DiagramStroke::Thin);
            let n = 5;
            for i in 1..n {
                let y = bottom + run * i as f64 / n as f64;
                s.line(
                    &[[-r, y], [r, y - run / n as f64 * 0.5]],
                    DiagramStroke::Thin,
                );
            }
            if run < drawn - 1e-6 {
                s.height(
                    "thread_length",
                    -r,
                    bottom,
                    top,
                    off,
                    format!("b {}", fmt(self.threaded())),
                );
            }
        }
        // The head.
        let k = self.k;
        match self.head {
            Head::SocketCap | Head::LowHead => {
                s.rect([-self.dk / 2.0, 0.0], [self.dk / 2.0, k]);
            }
            Head::Button => {
                let (rk, rf) = (self.dk / 2.0, self.flat_top());
                let (zc, radius) = self.dome();
                let a0 = (0.0 - zc).atan2(rk);
                let a1 = (k - zc).atan2(rf);
                let mut dome = geom::arc_points([0.0, zc], radius, a0, a1, 12);
                dome.extend(geom::arc_points(
                    [0.0, zc],
                    radius,
                    std::f64::consts::PI - a1,
                    std::f64::consts::PI - a0,
                    12,
                ));
                dome.push([-rk, 0.0]);
                dome.push([rk, 0.0]);
                s.poly(&dome, DiagramStroke::Outline, true);
            }
            Head::Countersunk => {
                let rk = self.dk / 2.0;
                s.poly(
                    &[[-r, 0.0], [r, 0.0], [rk, k], [-rk, k]],
                    DiagramStroke::Outline,
                    true,
                );
            }
            Head::Hex => {
                let e = across_corners(self.dk) / 2.0;
                let a = e / 2.0;
                s.rect([-e, 0.0], [e, k]);
                s.line(&[[-a, 0.0], [-a, k]], DiagramStroke::Outline);
                s.line(&[[a, 0.0], [a, k]], DiagramStroke::Outline);
            }
            Head::Thumb => {
                let (rs, rk, t) = (self.s / 2.0, self.dk / 2.0, self.t);
                s.rect([-rs, 0.0], [rs, k - t]);
                s.rect([-rk, k - t], [rk, k]);
                let n = 9;
                for i in 1..n {
                    let x = -rk + 2.0 * rk * i as f64 / n as f64;
                    s.line(&[[x, k - t], [x, k]], DiagramStroke::Thin);
                }
                s.height("t", rk, k - t, k, -off * 0.6, format!("t {}", fmt(t)));
                s.width("s", -rs, rs, 0.0, -off * 0.5, format!("ds {}", fmt(self.s)));
            }
            Head::Set => {
                let point = (self.d - self.dk) / 2.0;
                let (body, drawn_set, broken_set) = shank(0.0, r, 0.0, self.length);
                let _ = (drawn_set, broken_set);
                s.poly(&body, DiagramStroke::Outline, !broken_set);
                let b = -drawn_set;
                s.line(
                    &[[-r, b + point], [-self.dk / 2.0, b]],
                    DiagramStroke::Outline,
                );
                s.line(
                    &[[r, b + point], [self.dk / 2.0, b]],
                    DiagramStroke::Outline,
                );
            }
        }
        // The drive, behind the surface.
        if self.head.has_socket() && self.socket {
            let top = if self.head == Head::Set { 0.0 } else { k };
            let hs = self.s / 2.0;
            s.hidden(&[
                [-hs, top],
                [-hs, top - self.t],
                [hs, top - self.t],
                [hs, top],
            ]);
            let x = wide / 2.0 + off * 0.9;
            s.callout(
                "s",
                [hs, top - self.t / 2.0],
                [x + off * 0.8, top + off * 0.6],
                format!("s {}", fmt(self.s)),
            );
        }
        // The measures.
        let axis_top = if self.head == Head::Set { 0.0 } else { k };
        s.axis(0.0, bottom - off * 0.4, axis_top + off * 0.4);
        if self.head != Head::Set {
            let label = if self.head == Head::Hex { "s" } else { "dk" };
            s.width(
                "dk",
                -wide / 2.0,
                wide / 2.0,
                k,
                off,
                format!("{label} {}", fmt(self.dk)),
            );
            s.height("k", -wide / 2.0, 0.0, k, off, format!("k {}", fmt(k)));
        }
        let (l_top, l_bottom) = match self.head {
            Head::Countersunk => (k, bottom),
            Head::Set => (0.0, -shank(0.0, r, 0.0, self.length).1),
            _ => (0.0, bottom),
        };
        s.height(
            "length",
            wide / 2.0,
            l_top,
            l_bottom,
            -off,
            format!("L {}", fmt(self.length)),
        );
        let tip = l_bottom;
        let tip_r = if self.head == Head::Set {
            self.dk / 2.0
        } else {
            r
        };
        let _ = tip_r;
        s.width(
            "d",
            -r,
            r,
            tip,
            -off,
            format!("{} × {}", self.size, fmt(self.pitch)),
        );
        s.finish("screw")
    }
}

/// How many parameters a screw's panel binds, for the tests.
#[cfg(test)]
pub fn bound_fields(widgets: &[Widget]) -> usize {
    fn walk(widgets: &[Widget], n: &mut usize) {
        for w in widgets {
            match w {
                Widget::Number { bind: Some(_), .. } => *n += 1,
                Widget::Group { children, .. } => walk(children, n),
                _ => {}
            }
        }
    }
    let mut n = 0;
    walk(widgets, &mut n);
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_thumb_screw_is_a_knurled_disc_on_a_shoulder() {
        let screw = Screw::new(Head::Thumb, "M4", &Defaults::default());
        assert_eq!((screw.dk, screw.k, screw.s, screw.t), (16.0, 9.5, 8.0, 3.5));
        assert_eq!(screw.thread_length, 12.0, "three diameters");
        assert_eq!(screw.problem(), None);
        let roles: Vec<_> = screw.ops().iter().map(|op| op.boolean_op()).collect();
        assert_eq!(roles, [Some(BooleanOp::NewSolid), Some(BooleanOp::Fuse)]);
    }

    #[test]
    fn a_size_off_the_table_is_made_in_proportion() {
        let d = Defaults::default();
        let screw = Screw::with_args(&json!({"size": "M7"}), &d).unwrap();
        assert_eq!(
            (screw.size.as_str(), screw.d, screw.pitch),
            ("M7", 7.0, 1.0)
        );
        assert_eq!((screw.dk, screw.k, screw.s, screw.t), (10.5, 7.0, 5.6, 3.5));
        assert_eq!(screw.length, 25.0);
        assert_eq!(screw.problem(), None);
        let hex = Screw::with_args(&json!({"head": "hex_bolt", "d": 14.0}), &d).unwrap();
        assert_eq!(
            (hex.size.as_str(), hex.dk, hex.k, hex.pitch),
            ("M14", 22.9, 9.1, 2.0)
        );
        assert!(hex.custom);
        let listed = Screw::with_args(&json!({"d": 5.0}), &d).unwrap();
        assert_eq!((listed.size.as_str(), listed.dk), ("M5", 8.5));
        assert!(Screw::with_args(&json!({"size": "big"}), &d).is_err());
    }
    use printcad_bench_sdk::api::kernel_api::SweepKind;
    use printcad_bench_sdk::json;

    fn m3(head: Head) -> Screw {
        Screw::new(head, "M3", &Defaults::default())
    }

    #[test]
    fn a_new_cap_screw_is_sized_by_its_standard() {
        let screw = m3(Head::SocketCap);
        assert_eq!(screw.standard, "ISO 4762 / DIN 912");
        assert_eq!(
            (screw.d, screw.pitch, screw.dk, screw.k, screw.s),
            (3.0, 0.5, 5.5, 3.0, 2.5)
        );
        assert_eq!(screw.length, 10.0);
        assert_eq!(screw.thread_length, 18.0);
        assert_eq!(screw.label(), "M3 × 10 cap screw");
        assert_eq!(screw.problem(), None);
    }

    #[test]
    fn every_head_builds_its_body_and_its_drive() {
        for head in Head::ALL {
            let screw = m3(head);
            assert_eq!(screw.problem(), None, "{}", head.name());
            let ops = screw.ops();
            assert_eq!(
                ops[0].boolean_op(),
                Some(BooleanOp::NewSolid),
                "{}",
                head.name()
            );
            let cuts = ops
                .iter()
                .filter(|op| op.boolean_op() == Some(BooleanOp::Cut))
                .count();
            assert_eq!(cuts, usize::from(head.has_socket()), "{}", head.name());
            let ctx = Ctx {
                feature: "f",
                focus: Some("length"),
            };
            let panel = screw.panel(&ctx);
            let Some(Widget::Diagram { dimensions, .. }) = panel.first() else {
                panic!("{}: a drawing first", head.name());
            };
            assert!(
                dimensions
                    .iter()
                    .any(|d| d.emphasis && d.text.starts_with("L "))
            );
            assert_eq!(
                bound_fields(&panel),
                1,
                "{}: only the length until custom",
                head.name()
            );
        }
    }

    #[test]
    fn a_hex_bolt_is_a_shank_with_its_head_fused_on_and_crowned() {
        let ops = m3(Head::Hex).ops();
        let roles: Vec<_> = ops.iter().map(|op| op.boolean_op()).collect();
        assert_eq!(
            roles,
            [
                Some(BooleanOp::NewSolid),
                Some(BooleanOp::Fuse),
                Some(BooleanOp::Common)
            ]
        );
        let mut threaded = m3(Head::Hex);
        threaded.thread = true;
        let roles: Vec<_> = threaded.ops().iter().map(|op| op.boolean_op()).collect();
        assert_eq!(
            roles,
            [
                Some(BooleanOp::NewSolid),
                Some(BooleanOp::Cut),
                Some(BooleanOp::Fuse),
                Some(BooleanOp::Common)
            ],
            "the thread is cut before the head goes on"
        );
    }

    #[test]
    fn a_modelled_thread_runs_up_from_the_tip_and_stops_short_of_the_head() {
        let mut screw = m3(Head::SocketCap);
        screw.length = 20.0;
        screw.thread = true;
        let helix = |ops: &[SolidOp]| {
            ops.iter()
                .find_map(|op| match op {
                    SolidOp::Sweep {
                        kind: SweepKind::Helix { height, .. },
                        profile,
                        ..
                    } => Some((*height, profile.plane.origin[2])),
                    _ => None,
                })
                .expect("a helix among the ops")
        };
        let ops = screw.ops();
        assert_eq!(ops.len(), 3, "body, thread, then the recess");
        let (height, z_top) = helix(&ops);
        // 18 of thread, a pitch of lead-in and a pitch past the tip.
        assert!((height - 19.0).abs() < 1e-9, "{height}");
        assert!((z_top - -2.0).abs() < 1e-9, "starts 2 below the head");
        screw.length = 8.0;
        let (height, _) = helix(&screw.ops());
        assert!(
            (height - 8.25).abs() < 1e-9,
            "threaded to the head from half a pitch down: {height}"
        );
    }

    #[test]
    fn changing_the_standard_resizes_the_head_and_keeps_the_nearest_size() {
        let mut screw = Screw::new(Head::SocketCap, "M5", &Defaults::default());
        assert!(screw.apply(&PanelEvent::Choice {
            id: "standard".into(),
            index: 1,
        }));
        assert_eq!(screw.standard, "ASME B18.3 (inch)");
        assert_eq!(screw.size, "#10-24", "the nearest to 5 mm");
        assert!((screw.d - 4.826).abs() < 1e-3);
        assert!(screw.apply(&PanelEvent::Toggle {
            id: "custom".into(),
            on: true,
        }));
        assert!(screw.apply(&PanelEvent::Number {
            id: "dk".into(),
            value: 9.0,
        }));
        assert_eq!(screw.dk, 9.0);
        assert!(screw.apply(&PanelEvent::Toggle {
            id: "custom".into(),
            on: false,
        }));
        assert!((screw.dk - 0.312 * 25.4).abs() < 1e-9, "back to the table");
    }

    #[test]
    fn a_command_names_its_screw() {
        let screw = Screw::with_args(
            &json!({"head": "button", "size": "M4", "length": 16, "thread": true}),
            &Defaults::default(),
        )
        .unwrap();
        assert_eq!(screw.head, Head::Button);
        assert_eq!((screw.dk, screw.length), (7.6, 16.0));
        assert!(screw.thread);
        assert!(Screw::with_args(&json!({"head": "pan"}), &Defaults::default()).is_err());
        assert!(Screw::with_args(&json!({"size": "7 mm"}), &Defaults::default()).is_err());
        let custom = Screw::with_args(&json!({"dk": 6.0}), &Defaults::default()).unwrap();
        assert!(custom.custom && custom.dk == 6.0);
    }

    #[test]
    fn a_screw_saved_without_the_newer_fields_reads() {
        let old = json!({
            "head": "socket_cap", "standard": "ISO 4762 / DIN 912", "size": "M3", "length": 10.0,
            "d": 3.0, "pitch": 0.5, "dk": 5.5, "k": 3.0, "s": 2.5, "t": 1.3
        });
        let screw: Screw = printcad_bench_sdk::serde_json::from_value(old).unwrap();
        assert!(screw.socket && !screw.thread && !screw.custom);
        assert_eq!(screw.thread_length, 0.0);
    }

    #[test]
    fn a_socket_deeper_than_its_head_is_refused() {
        let mut screw = m3(Head::Button);
        screw.t = 5.0;
        assert!(screw.problem().is_some());
        let mut screw = m3(Head::Countersunk);
        screw.length = 1.0;
        assert!(screw.problem().is_some());
    }
}
