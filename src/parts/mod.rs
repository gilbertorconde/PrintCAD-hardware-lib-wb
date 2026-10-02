//! The hardware families, each a feature kind of its own with the data
//! it is built from, the panel it is edited in and the kernel ops that
//! make it.

pub mod bearing;
pub mod extrusion;
pub mod gear;
pub mod insert;
pub mod magnet;
pub mod nut;
pub mod rod;
pub mod screw;
pub mod spring;
pub mod standoff;
pub mod tnut;
pub mod washer;

use printcad_bench_sdk::api::kernel_api::SolidOp;
use printcad_bench_sdk::api::{Bind, Dim, NoteKind, PanelEvent, Parameter, Widget};
use printcad_bench_sdk::{Value, serde_json};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub use bearing::Bearing;
pub use extrusion::Extrusion;
pub use gear::Gear;
pub use insert::Insert;
pub use magnet::Magnet;
pub use nut::Nut;
pub use rod::Rod;
pub use screw::Screw;
pub use spring::Spring;
pub use standoff::Standoff;
pub use tnut::TNut;
pub use washer::Washer;

/// The package id every kind, tool and command is named under.
pub const PACKAGE: &str = "io.github.gilbertorconde.hardware";

/// What new parts start from, kept as the bench's settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Defaults {
    /// The thread size new fasteners take, `M3`.
    pub size: String,
    /// Whether new screws carry their drive recess.
    pub socket: bool,
    /// Whether new fasteners carry a modelled thread.
    pub thread: bool,
    /// The extrusion series new extrusions take: 20, 30 or 40.
    pub series: u32,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            size: "M3".into(),
            socket: true,
            thread: false,
            series: 20,
        }
    }
}

/// What a panel is drawn for.
pub struct Ctx<'a> {
    /// The feature under edit, for the numbers' bindings.
    pub feature: &'a str,
    /// The field last changed, whose measure the drawing emphasises.
    pub focus: Option<&'a str>,
}

/// A hardware family.
pub trait Part: Clone + Serialize + DeserializeOwned {
    const FAMILY: Family;
    /// The part named for the tree: `M3 × 10 cap screw`.
    fn label(&self) -> String;
    fn icon(&self) -> &'static str;
    /// What stops it being built, if anything.
    fn problem(&self) -> Option<String>;
    /// The ops that build it; the first starts the solid.
    fn ops(&self) -> Vec<SolidOp>;
    /// Where it runs along its axis, in its body's frame: `(from, to)`.
    fn axis(&self) -> ([f64; 3], [f64; 3]);
    fn panel(&self, ctx: &Ctx) -> Vec<Widget>;
    /// The numbers formulas may set.
    fn parameters(&self) -> Vec<Parameter>;
    /// Take a panel change; `true` when the data changed.
    fn apply(&mut self, event: &PanelEvent) -> bool;
    /// One from a command's arguments, the rest from `defaults`.
    fn with_args(args: &Value, defaults: &Defaults) -> Result<Self, String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Screw,
    Nut,
    TNut,
    Washer,
    Extrusion,
    Insert,
    Bearing,
    Magnet,
    Rod,
    Spring,
    Standoff,
    Gear,
}

impl Family {
    pub const ALL: [Family; 12] = [
        Family::Screw,
        Family::Nut,
        Family::TNut,
        Family::Washer,
        Family::Extrusion,
        Family::Insert,
        Family::Bearing,
        Family::Magnet,
        Family::Rod,
        Family::Spring,
        Family::Standoff,
        Family::Gear,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Family::Screw => "screw",
            Family::Nut => "nut",
            Family::TNut => "tnut",
            Family::Washer => "washer",
            Family::Extrusion => "extrusion",
            Family::Insert => "insert",
            Family::Bearing => "bearing",
            Family::Magnet => "magnet",
            Family::Rod => "rod",
            Family::Spring => "spring",
            Family::Standoff => "standoff",
            Family::Gear => "gear",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Family::Screw => "Screw",
            Family::Nut => "Nut",
            Family::TNut => "T-slot nut",
            Family::Washer => "Washer",
            Family::Extrusion => "Extrusion",
            Family::Insert => "Heat-set insert",
            Family::Bearing => "Bearing",
            Family::Magnet => "Magnet",
            Family::Rod => "Rod",
            Family::Spring => "Spring",
            Family::Standoff => "Standoff",
            Family::Gear => "Gear",
        }
    }

    /// The feature kind, `io.github….hardware.screw`.
    pub fn kind(self) -> String {
        format!("{PACKAGE}.{}", self.name())
    }

    pub fn from_kind(kind: &str) -> Option<Family> {
        Family::ALL.into_iter().find(|f| f.kind() == kind)
    }

    pub fn named(name: &str) -> Option<Family> {
        Family::ALL
            .into_iter()
            .find(|f| f.name().eq_ignore_ascii_case(name))
    }
}

/// A part of any family.
#[derive(Debug, Clone, PartialEq)]
pub enum Hardware {
    Screw(Screw),
    Nut(Nut),
    TNut(TNut),
    Washer(Washer),
    Extrusion(Extrusion),
    Insert(Insert),
    Bearing(Bearing),
    Magnet(Magnet),
    Rod(Rod),
    Spring(Spring),
    Standoff(Standoff),
    Gear(Gear),
}

macro_rules! each {
    ($self:expr, $p:ident => $body:expr) => {
        match $self {
            Hardware::Screw($p) => $body,
            Hardware::Nut($p) => $body,
            Hardware::TNut($p) => $body,
            Hardware::Washer($p) => $body,
            Hardware::Extrusion($p) => $body,
            Hardware::Insert($p) => $body,
            Hardware::Bearing($p) => $body,
            Hardware::Magnet($p) => $body,
            Hardware::Rod($p) => $body,
            Hardware::Spring($p) => $body,
            Hardware::Standoff($p) => $body,
            Hardware::Gear($p) => $body,
        }
    };
}

fn read<P: Part>(data: &Value) -> Option<P> {
    serde_json::from_value(data.clone()).ok()
}

impl Hardware {
    /// A part of `kind` read from a feature's data.
    pub fn read(kind: &str, data: &Value) -> Option<Hardware> {
        Some(match Family::from_kind(kind)? {
            Family::Screw => Hardware::Screw(read(data)?),
            Family::Nut => Hardware::Nut(read(data)?),
            Family::TNut => Hardware::TNut(read(data)?),
            Family::Washer => Hardware::Washer(read(data)?),
            Family::Extrusion => Hardware::Extrusion(read(data)?),
            Family::Insert => Hardware::Insert(read(data)?),
            Family::Bearing => Hardware::Bearing(read(data)?),
            Family::Magnet => Hardware::Magnet(read(data)?),
            Family::Rod => Hardware::Rod(read(data)?),
            Family::Spring => Hardware::Spring(read(data)?),
            Family::Standoff => Hardware::Standoff(read(data)?),
            Family::Gear => Hardware::Gear(read(data)?),
        })
    }

    /// A part from a command's arguments.
    pub fn with_args(
        family: Family,
        args: &Value,
        defaults: &Defaults,
    ) -> Result<Hardware, String> {
        Ok(match family {
            Family::Screw => Hardware::Screw(Screw::with_args(args, defaults)?),
            Family::Nut => Hardware::Nut(Nut::with_args(args, defaults)?),
            Family::TNut => Hardware::TNut(TNut::with_args(args, defaults)?),
            Family::Washer => Hardware::Washer(Washer::with_args(args, defaults)?),
            Family::Extrusion => Hardware::Extrusion(Extrusion::with_args(args, defaults)?),
            Family::Insert => Hardware::Insert(Insert::with_args(args, defaults)?),
            Family::Bearing => Hardware::Bearing(Bearing::with_args(args, defaults)?),
            Family::Magnet => Hardware::Magnet(Magnet::with_args(args, defaults)?),
            Family::Rod => Hardware::Rod(Rod::with_args(args, defaults)?),
            Family::Spring => Hardware::Spring(Spring::with_args(args, defaults)?),
            Family::Standoff => Hardware::Standoff(Standoff::with_args(args, defaults)?),
            Family::Gear => Hardware::Gear(Gear::with_args(args, defaults)?),
        })
    }

    pub fn family(&self) -> Family {
        each!(self, p => family_of(p))
    }

    pub fn kind(&self) -> String {
        self.family().kind()
    }

    pub fn to_value(&self) -> Value {
        each!(self, p => serde_json::to_value(p).unwrap_or(Value::Null))
    }

    pub fn label(&self) -> String {
        each!(self, p => p.label())
    }

    pub fn icon(&self) -> &'static str {
        each!(self, p => p.icon())
    }

    pub fn problem(&self) -> Option<String> {
        each!(self, p => p.problem())
    }

    pub fn ops(&self) -> Vec<SolidOp> {
        each!(self, p => p.ops())
    }

    pub fn axis(&self) -> ([f64; 3], [f64; 3]) {
        each!(self, p => p.axis())
    }

    pub fn panel(&self, ctx: &Ctx) -> Vec<Widget> {
        each!(self, p => p.panel(ctx))
    }

    pub fn parameters(&self) -> Vec<Parameter> {
        each!(self, p => p.parameters())
    }

    pub fn apply(&mut self, event: &PanelEvent) -> bool {
        each!(self, p => p.apply(event))
    }
}

fn family_of<P: Part>(_: &P) -> Family {
    P::FAMILY
}

// ------------------------------------------------------------- helpers

/// A number bound to the feature's field `id`, so it takes formulas.
pub fn number(ctx: &Ctx, id: &str, label: &str, value: f64, min: f64, decimals: usize) -> Widget {
    Widget::Number {
        id: id.into(),
        label: label.into(),
        value,
        dim: Dim::Length,
        bind: Some(Bind {
            feature: ctx.feature.into(),
            key: format!("/{id}"),
        }),
        min: Some(min),
        max: None,
        decimals,
        error: None,
    }
}

/// A count bound to the feature's field `id`.
pub fn count(ctx: &Ctx, id: &str, label: &str, value: f64, min: f64) -> Widget {
    Widget::Number {
        id: id.into(),
        label: label.into(),
        value,
        dim: Dim::Number,
        bind: Some(Bind {
            feature: ctx.feature.into(),
            key: format!("/{id}"),
        }),
        min: Some(min),
        max: None,
        decimals: 0,
        error: None,
    }
}

pub fn choice<S: AsRef<str>>(id: &str, label: &str, options: &[S], selected: usize) -> Widget {
    Widget::Choice {
        id: id.into(),
        label: label.into(),
        options: options.iter().map(|o| o.as_ref().to_string()).collect(),
        selected,
    }
}

pub fn toggle(id: &str, label: &str, on: bool) -> Widget {
    Widget::Toggle {
        id: id.into(),
        label: label.into(),
        on,
    }
}

pub fn group(title: &str, open: bool, children: Vec<Widget>) -> Widget {
    Widget::Group {
        title: title.into(),
        open,
        children,
    }
}

pub fn text(text: impl Into<String>) -> Widget {
    Widget::Text {
        text: text.into(),
        mono: false,
    }
}

pub fn note(problem: Option<String>) -> Option<Widget> {
    problem.map(|text| Widget::Note {
        kind: NoteKind::Error,
        title: None,
        text,
    })
}

/// A length parameter at `/key`.
pub fn length(key: &str, label: &str) -> Parameter {
    Parameter {
        key: format!("/{key}"),
        name: Some(key.into()),
        label: label.into(),
        dim: Dim::Length,
        pointer: format!("/{key}"),
        scale: 1.0,
        integer: false,
    }
}

/// A count parameter at `/key`.
pub fn integer(key: &str, label: &str) -> Parameter {
    Parameter {
        key: format!("/{key}"),
        name: Some(key.into()),
        label: label.into(),
        dim: Dim::Number,
        pointer: format!("/{key}"),
        scale: 1.0,
        integer: true,
    }
}

/// The index of `name` among `options`, or 0.
pub fn index_of<S: AsRef<str>>(options: &[S], name: &str) -> usize {
    options
        .iter()
        .position(|o| o.as_ref().eq_ignore_ascii_case(name))
        .unwrap_or(0)
}

pub fn arg_f64(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(Value::as_f64)
}

pub fn arg_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

pub fn arg_bool(args: &Value, key: &str) -> Option<bool> {
    args.get(key).and_then(Value::as_bool)
}

/// A number for a label, without trailing zeros.
pub fn fmt(v: f64) -> String {
    crate::geom::trim(v)
}

/// The holes made for a thread, for a panel: its clearance holes, its
/// tap drill and the hole for its heat-set insert.
pub fn holes_text(d: f64, pitch: f64) -> String {
    let mut parts = Vec::new();
    if let Some(h) = crate::standards::clearance(d) {
        parts.push(format!(
            "clearance Ø{} (fine Ø{}, coarse Ø{})",
            fmt(h.medium),
            fmt(h.fine),
            fmt(h.coarse)
        ));
    } else {
        parts.push(format!(
            "clearance about Ø{}",
            fmt(((d * 1.1) * 10.0).round() / 10.0)
        ));
    }
    if pitch > 0.0 && pitch < d {
        parts.push(format!(
            "tap drill Ø{}",
            fmt(((d - pitch) * 100.0).round() / 100.0)
        ));
    }
    if let Some(i) = crate::standards::insert_for(d) {
        parts.push(format!(
            "heat-set insert Ø{} × {} deep",
            fmt(i.hole),
            fmt(i.length + 1.0)
        ));
    }
    let mut text = parts.join(" · ");
    if let Some(first) = text.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    text
}

/// An axis up Z from `z0` to `z1`.
pub fn z_axis(z0: f64, z1: f64) -> ([f64; 3], [f64; 3]) {
    ([0.0, 0.0, z0], [0.0, 0.0, z1])
}

/// The standard screw lengths, from which a new fastener's length is
/// chosen: the first at or over three diameters.
pub const LENGTHS: [f64; 21] = [
    4.0, 5.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 20.0, 25.0, 30.0, 35.0, 40.0, 45.0, 50.0, 55.0,
    60.0, 70.0, 80.0, 90.0, 100.0,
];

pub fn default_length(d: f64) -> f64 {
    let want = 3.0 * d;
    LENGTHS
        .iter()
        .copied()
        .find(|l| *l >= want)
        .unwrap_or(want.round())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_are_named_under_the_package_and_read_back() {
        for family in Family::ALL {
            assert!(family.kind().starts_with(PACKAGE));
            assert_eq!(Family::from_kind(&family.kind()), Some(family));
            assert_eq!(Family::named(family.name()), Some(family));
        }
        assert_eq!(Family::from_kind("other.kind"), None);
    }

    #[test]
    fn the_holes_for_a_thread_are_named() {
        assert_eq!(
            holes_text(3.0, 0.5),
            "Clearance Ø3.4 (fine Ø3.2, coarse Ø3.6) · tap drill Ø2.5 · heat-set insert Ø4 × 6.7 deep"
        );
        assert_eq!(holes_text(7.0, 1.0), "Clearance about Ø7.7 · tap drill Ø6");
    }

    #[test]
    fn a_new_fastener_is_three_diameters_long_or_the_next_standard_length() {
        assert_eq!(default_length(3.0), 10.0);
        assert_eq!(default_length(4.0), 12.0);
        assert_eq!(default_length(8.0), 25.0);
        assert_eq!(default_length(40.0), 120.0);
    }

    #[test]
    fn every_family_reads_from_a_command_with_no_arguments_and_round_trips() {
        let defaults = Defaults::default();
        for family in Family::ALL {
            let part = Hardware::with_args(family, &Value::Null, &defaults)
                .unwrap_or_else(|e| panic!("{}: {e}", family.name()));
            assert_eq!(part.family(), family);
            assert_eq!(part.problem(), None, "{}", family.name());
            assert!(!part.ops().is_empty(), "{}", family.name());
            let back = Hardware::read(&part.kind(), &part.to_value()).expect("reads back");
            assert_eq!(back, part);
            let ctx = Ctx {
                feature: "f",
                focus: None,
            };
            let panel = part.panel(&ctx);
            assert!(
                matches!(panel.first(), Some(Widget::Diagram { .. })),
                "{} opens with its drawing",
                family.name()
            );
        }
    }
}
