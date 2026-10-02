//! Hardware: a printCAD workbench package that adds standard hardware to
//! a design, sized from its standards. A tool per part opens a panel
//! with a drawing of the part, its standard and size, and whatever else
//! it has to set; the part appears on a body of its own as it is set.
//!
//! Each family (screws, nuts, washers, T-slot extrusions, heat-set
//! inserts, bearings, magnets, rods, springs) is a feature kind of the
//! package; `parts` holds them, `standards` the tables they are sized
//! from, `geom` the kernel ops they build with and `diagram` the drawing
//! at the top of the panel.

pub mod diagram;
pub mod geom;
pub mod parts;
pub mod standards;

use printcad_bench_sdk::api::*;
use printcad_bench_sdk::{Bench, Value, bench, host, json, serde_json};

use parts::bearing::BearingKind;
use parts::nut::NutKind;
use parts::rod::RodKind;
use parts::screw::Head;
use parts::tnut::TNutKind;
use parts::washer::WasherKind;
use parts::{
    Bearing, Ctx, Defaults, Extrusion, Family, Hardware, Insert, Magnet, Nut, PACKAGE, Rod, Screw,
    Spring, TNut, Washer,
};

/// A tool of the toolbar: what it makes.
struct ToolDef {
    suffix: &'static str,
    label: &'static str,
    icon: &'static str,
    category: &'static str,
    row: u8,
    seed: fn(&Defaults) -> Hardware,
}

macro_rules! screw_tool {
    ($name:ident, $head:expr) => {
        fn $name(d: &Defaults) -> Hardware {
            Hardware::Screw(Screw::new($head, &d.size, d))
        }
    };
}

macro_rules! nut_tool {
    ($name:ident, $kind:expr) => {
        fn $name(d: &Defaults) -> Hardware {
            Hardware::Nut(Nut::new($kind, &d.size, d))
        }
    };
}

screw_tool!(seed_socket_cap, Head::SocketCap);
screw_tool!(seed_button, Head::Button);
screw_tool!(seed_countersunk, Head::Countersunk);
screw_tool!(seed_hex_bolt, Head::Hex);
screw_tool!(seed_low_head, Head::LowHead);
screw_tool!(seed_set_screw, Head::Set);
nut_tool!(seed_hex_nut, NutKind::Hex);
nut_tool!(seed_thin_nut, NutKind::Thin);
nut_tool!(seed_nyloc_nut, NutKind::Nyloc);
nut_tool!(seed_square_nut, NutKind::Square);

macro_rules! tnut_tool {
    ($name:ident, $kind:expr) => {
        fn $name(d: &Defaults) -> Hardware {
            Hardware::TNut(TNut::new($kind, d.series, &d.size, d))
        }
    };
}

tnut_tool!(seed_sliding_tnut, TNutKind::Sliding);
tnut_tool!(seed_drop_in_tnut, TNutKind::DropIn);
tnut_tool!(seed_spring_tnut, TNutKind::SpringBall);
tnut_tool!(seed_twist_tnut, TNutKind::Twist);
tnut_tool!(seed_roll_in_tnut, TNutKind::RollIn);

fn seed_washer(d: &Defaults) -> Hardware {
    Hardware::Washer(Washer::new(WasherKind::Flat, &d.size))
}
fn seed_large_washer(d: &Defaults) -> Hardware {
    Hardware::Washer(Washer::new(WasherKind::Large, &d.size))
}
fn seed_spring_washer(d: &Defaults) -> Hardware {
    Hardware::Washer(Washer::new(WasherKind::Spring, &d.size))
}
fn seed_extrusion(d: &Defaults) -> Hardware {
    Hardware::Extrusion(Extrusion::new(d.series, 1, 1, 100.0))
}
fn seed_insert(d: &Defaults) -> Hardware {
    Hardware::Insert(Insert::new(&d.size, d))
}
fn seed_bearing(_: &Defaults) -> Hardware {
    Hardware::Bearing(Bearing::new(BearingKind::Ball, "608"))
}
fn seed_linear_bearing(_: &Defaults) -> Hardware {
    Hardware::Bearing(Bearing::new(BearingKind::Linear, "LM8UU"))
}
fn seed_magnet(_: &Defaults) -> Hardware {
    Hardware::Magnet(Magnet::new(parts::magnet::MagnetShape::Disc))
}
fn seed_shaft(d: &Defaults) -> Hardware {
    Hardware::Rod(Rod::new(RodKind::Shaft, "Ø8", d))
}
fn seed_threaded_rod(d: &Defaults) -> Hardware {
    Hardware::Rod(Rod::new(RodKind::Threaded, &d.size, d))
}
fn seed_spring(_: &Defaults) -> Hardware {
    Hardware::Spring(Spring::default())
}

const TOOLS: &[ToolDef] = &[
    ToolDef {
        suffix: "socket_cap",
        label: "Socket head cap screw",
        icon: "screw-socket",
        category: "Screws",
        row: 1,
        seed: seed_socket_cap,
    },
    ToolDef {
        suffix: "button",
        label: "Button head screw",
        icon: "screw-button",
        category: "Screws",
        row: 1,
        seed: seed_button,
    },
    ToolDef {
        suffix: "countersunk",
        label: "Countersunk screw",
        icon: "screw-countersunk",
        category: "Screws",
        row: 1,
        seed: seed_countersunk,
    },
    ToolDef {
        suffix: "hex_bolt",
        label: "Hex bolt",
        icon: "screw-hex",
        category: "Screws",
        row: 1,
        seed: seed_hex_bolt,
    },
    ToolDef {
        suffix: "low_head",
        label: "Low head cap screw",
        icon: "screw-low-head",
        category: "Screws",
        row: 1,
        seed: seed_low_head,
    },
    ToolDef {
        suffix: "set_screw",
        label: "Set screw",
        icon: "screw-set",
        category: "Screws",
        row: 1,
        seed: seed_set_screw,
    },
    ToolDef {
        suffix: "hex_nut",
        label: "Hex nut",
        icon: "nut-hex",
        category: "Nuts",
        row: 1,
        seed: seed_hex_nut,
    },
    ToolDef {
        suffix: "thin_nut",
        label: "Thin nut",
        icon: "nut-thin",
        category: "Nuts",
        row: 1,
        seed: seed_thin_nut,
    },
    ToolDef {
        suffix: "nyloc_nut",
        label: "Nylon insert lock nut",
        icon: "nut-nyloc",
        category: "Nuts",
        row: 1,
        seed: seed_nyloc_nut,
    },
    ToolDef {
        suffix: "square_nut",
        label: "Square nut",
        icon: "nut-square",
        category: "Nuts",
        row: 1,
        seed: seed_square_nut,
    },
    ToolDef {
        suffix: "washer",
        label: "Washer",
        icon: "washer-flat",
        category: "Washers",
        row: 1,
        seed: seed_washer,
    },
    ToolDef {
        suffix: "large_washer",
        label: "Large washer",
        icon: "washer-large",
        category: "Washers",
        row: 1,
        seed: seed_large_washer,
    },
    ToolDef {
        suffix: "spring_washer",
        label: "Spring washer",
        icon: "washer-spring",
        category: "Washers",
        row: 1,
        seed: seed_spring_washer,
    },
    ToolDef {
        suffix: "extrusion",
        label: "T-slot extrusion",
        icon: "extrusion",
        category: "T-slot",
        row: 2,
        seed: seed_extrusion,
    },
    ToolDef {
        suffix: "sliding_tnut",
        label: "Sliding T-nut",
        icon: "tnut-sliding",
        category: "T-slot",
        row: 2,
        seed: seed_sliding_tnut,
    },
    ToolDef {
        suffix: "drop_in_tnut",
        label: "Drop-in T-nut",
        icon: "tnut-drop-in",
        category: "T-slot",
        row: 2,
        seed: seed_drop_in_tnut,
    },
    ToolDef {
        suffix: "spring_tnut",
        label: "Spring-ball T-nut",
        icon: "tnut-spring",
        category: "T-slot",
        row: 2,
        seed: seed_spring_tnut,
    },
    ToolDef {
        suffix: "twist_tnut",
        label: "Twist T-nut",
        icon: "tnut-twist",
        category: "T-slot",
        row: 2,
        seed: seed_twist_tnut,
    },
    ToolDef {
        suffix: "roll_in_tnut",
        label: "Roll-in T-nut",
        icon: "tnut-roll-in",
        category: "T-slot",
        row: 2,
        seed: seed_roll_in_tnut,
    },
    ToolDef {
        suffix: "insert",
        label: "Heat-set insert",
        icon: "insert",
        category: "Inserts",
        row: 2,
        seed: seed_insert,
    },
    ToolDef {
        suffix: "bearing",
        label: "Ball bearing",
        icon: "bearing",
        category: "Bearings",
        row: 2,
        seed: seed_bearing,
    },
    ToolDef {
        suffix: "linear_bearing",
        label: "Linear bushing",
        icon: "bearing-linear",
        category: "Bearings",
        row: 2,
        seed: seed_linear_bearing,
    },
    ToolDef {
        suffix: "shaft",
        label: "Smooth shaft",
        icon: "rod",
        category: "Rods",
        row: 2,
        seed: seed_shaft,
    },
    ToolDef {
        suffix: "threaded_rod",
        label: "Threaded rod",
        icon: "rod-threaded",
        category: "Rods",
        row: 2,
        seed: seed_threaded_rod,
    },
    ToolDef {
        suffix: "magnet",
        label: "Magnet",
        icon: "magnet",
        category: "Other",
        row: 2,
        seed: seed_magnet,
    },
    ToolDef {
        suffix: "spring",
        label: "Compression spring",
        icon: "spring",
        category: "Other",
        row: 2,
        seed: seed_spring,
    },
];

fn id(suffix: &str) -> String {
    format!("{PACKAGE}.{suffix}")
}

/// Every numeric field of every family that is a length, for the
/// property panel to show in the document's unit.
const LENGTH_KEYS: [&str; 37] = [
    "length",
    "d",
    "pitch",
    "dk",
    "k",
    "s",
    "t",
    "thread_length",
    "m",
    "collar_d",
    "collar_h",
    "d1",
    "d2",
    "h",
    "opening",
    "lip",
    "cavity",
    "depth",
    "hole",
    "corner",
    "outer",
    "bore",
    "width",
    "l",
    "w",
    "wire",
    "lead",
    "shoulder",
    "pilot",
    "top",
    "bottom",
    "thick",
    "straight",
    "thread_at",
    "floor",
    "neck",
    "neck_w",
];

#[derive(Default)]
struct HardwareBench {
    /// The part whose task is open, with its data when it opened.
    editing: Option<(String, Hardware)>,
    /// The tool just made it: Cancel removes it.
    fresh: bool,
    /// The field last changed, whose measure the drawing emphasises.
    focus: Option<String>,
    defaults: Defaults,
}

impl HardwareBench {
    /// A body with the part on it: `(body, feature)`.
    fn make(&self, part: &Hardware) -> Result<(String, String), String> {
        let body = host::create_body(Some(&part.label()))?;
        let feature = host::add_feature(&part.kind(), &part.label(), Some(&body), part.to_value())?;
        Ok((body, feature))
    }

    fn open(&mut self, id: &str, fresh: bool) {
        if let Some(node) = host::feature(id)
            && let Some(part) = Hardware::read(&node.kind, &node.data)
        {
            self.editing = Some((id.to_string(), part));
            self.fresh = fresh;
            self.focus = None;
        }
    }

    /// The part under edit as it is now.
    fn edited(&self) -> Option<(String, Hardware)> {
        let (id, _) = self.editing.as_ref()?;
        let node = host::feature(id)?;
        Some((id.clone(), Hardware::read(&node.kind, &node.data)?))
    }

    /// Make `part` on a body of its own and open it, fresh.
    fn add(&mut self, part: Hardware, journal: &str) {
        match self.make(&part) {
            Ok((body, feature)) => {
                host::request(Request::SelectBody { body });
                host::request(Request::JournalLabel {
                    label: journal.into(),
                });
                self.open(&feature, true);
            }
            Err(e) => host::error(&format!("No {}: {e}", part.family().name())),
        }
    }

    fn write(&self, id: &str, part: &Hardware) {
        if let Err(e) = host::set_feature_data(id, part.to_value()) {
            host::error(&e);
            return;
        }
        if let Some(node) = host::feature(id)
            && node.name != part.label()
        {
            let _ = host::call(
                calls::RENAME_FEATURE,
                json!({"id": id, "name": part.label()}),
            );
        }
    }

    /// What there is: every family's kinds, standards and sizes.
    fn catalog() -> Value {
        let screw: Vec<Value> = Head::ALL
            .iter()
            .map(|h| {
                json!({
                    "head": h.tool(),
                    "standards": h.standards().iter().map(|t| json!({
                        "name": t.name,
                        "sizes": t.rows.iter().map(|r| r.size).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        let nut: Vec<Value> = NutKind::ALL
            .iter()
            .map(|k| {
                let standards: Vec<Value> = k
                    .standards()
                    .iter()
                    .map(|t| json!({"name": t.name, "sizes": t.rows.iter().map(|r| r.size).collect::<Vec<_>>()}))
                    .collect();
                json!({"kind": k.tool(), "standards": standards})
            })
            .collect();
        let tnut: Vec<Value> = TNutKind::ALL
            .iter()
            .map(|k| {
                let series: Vec<Value> = standards::T_NUTS
                    .iter()
                    .filter(|r| r.kind == *k)
                    .map(|r| json!({"series": r.series, "sizes": r.sizes}))
                    .collect();
                json!({"kind": k.tool(), "series": series})
            })
            .collect();
        let washer: Vec<Value> = WasherKind::ALL
            .iter()
            .map(|k| {
                json!({
                    "kind": k.tool(),
                    "standard": k.table().name,
                    "sizes": k.table().rows.iter().map(|r| r.size).collect::<Vec<_>>(),
                })
            })
            .collect();
        json!({
            "screw": screw,
            "nut": nut,
            "tnut": tnut,
            "washer": washer,
            "extrusion": {
                "series": standards::EXTRUSIONS.iter().map(|s| s.cell).collect::<Vec<_>>(),
                "along": parts::extrusion::AXES,
                "slots": ["all", "three", "adjacent", "opposite", "one"],
            },
            "insert": standards::INSERTS.iter().map(|r| r.size).collect::<Vec<_>>(),
            "bearing": {
                "ball": standards::BALL_BEARINGS.iter().map(|r| r.name).collect::<Vec<_>>(),
                "linear": standards::LINEAR_BEARINGS.iter().map(|r| r.name).collect::<Vec<_>>(),
            },
            "magnet": {"shapes": ["disc", "ring", "block"]},
            "rod": RodKind::ALL.iter().map(|k| json!({"kind": k.tool(), "sizes": k.sizes()})).collect::<Vec<_>>(),
            "spring": {"fields": ["outer", "wire", "length", "turns"]},
        })
    }
}

const MAKE: &str = "make";
const CATALOG: &str = "catalog";
const EDIT: &str = "edit";
const ANOTHER: &str = "another";

impl Bench for HardwareBench {
    fn describe(&self) -> Registration {
        let param = |name: &str, kind: ParamKind, required: bool, doc: &str| Param {
            name: name.into(),
            kind,
            required,
            doc: doc.into(),
        };
        Registration {
            label: "Hardware".into(),
            description: "Standard hardware sized from its standards: screws, nuts, washers, extrusions, inserts, bearings, magnets, rods and springs".into(),
            icon: "hardware".into(),
            tools: TOOLS
                .iter()
                .map(|t| Tool {
                    id: id(t.suffix),
                    label: t.label.into(),
                    icon: t.icon.into(),
                    behavior: ToolBehavior::Action,
                    row: t.row,
                    category: Some(t.category.into()),
                    ..Default::default()
                })
                .collect(),
            commands: vec![
                Command {
                    id: id(MAKE),
                    summary: "Make a part on a body of its own".into(),
                    params: vec![
                        param("part", ParamKind::String, true, "`screw`, `nut`, `tnut`, `washer`, `extrusion`, `insert`, `bearing`, `magnet`, `rod` or `spring`"),
                        param("head", ParamKind::String, false, "a screw's head: `socket_cap` (the default), `button`, `countersunk`, `hex_bolt`, `low_head` or `set_screw`"),
                        param("kind", ParamKind::String, false, "a nut's (`hex`, `thin`, `nyloc`, `square`), T-slot nut's (`sliding`, `drop_in`, `spring_ball`, `twist`, `roll_in`), washer's (`flat`, `large`, `spring`), bearing's (`ball`, `linear`) or rod's (`shaft`, `threaded_rod`, `lead_screw`, `dowel_pin`) kind"),
                        param("standard", ParamKind::String, false, "the standard to size by, as `catalog` names it; the first when left out"),
                        param("size", ParamKind::String, false, "the thread size, `M3`; the Preferences default when left out"),
                        param("length", ParamKind::Number, false, "mm; a screw's from under its head"),
                        param("thread", ParamKind::Bool, false, "model the thread"),
                        param("socket", ParamKind::Bool, false, "model a screw's drive recess"),
                        param("series", ParamKind::Integer, false, "an extrusion's or T-nut's series: 20, 30 or 40"),
                        param("profile", ParamKind::String, false, "an extrusion's profile, `2040`"),
                        param("along", ParamKind::String, false, "an extrusion's axis: `X`, `Y` or `Z`"),
                        param("slots", ParamKind::String, false, "an extrusion's slotted faces: `all`, `three`, `adjacent`, `opposite` or `one`"),
                        param("name", ParamKind::String, false, "a bearing's designation, `608` or `LM8UU`"),
                        param("shape", ParamKind::String, false, "a magnet's shape: `disc`, `ring` or `block`"),
                        param("d", ParamKind::Number, false, "any dimension the part's panel shows may be given by its name (`d`, `dk`, `k`, `s`, `m`, `outer`, `wire`, `turns`…) and makes the part custom"),
                    ],
                    returns: "{body, feature}".into(),
                    read_only: false,
                },
                Command {
                    id: id(CATALOG),
                    summary: "The kinds, standards and sizes the package offers".into(),
                    params: Vec::new(),
                    returns: "{screw, nut, washer, extrusion, insert, bearing, magnet, rod, spring}".into(),
                    read_only: true,
                },
            ],
            length_keys: LENGTH_KEYS.map(String::from).to_vec(),
            ..Default::default()
        }
    }

    fn feature_info(&self, node: &Node) -> FeatureInfo {
        let family = Family::from_kind(&node.kind);
        match Hardware::read(&node.kind, &node.data) {
            Some(part) => FeatureInfo {
                icon: part.icon().into(),
                kind_label: part.label(),
                family_label: part.family().title().into(),
                builds_solid: true,
            },
            None => FeatureInfo {
                icon: "hardware".into(),
                kind_label: family.map_or("Hardware", |f| f.title()).into(),
                family_label: "Hardware".into(),
                builds_solid: true,
            },
        }
    }

    fn parameters(&self, node: &Node) -> Vec<Parameter> {
        Hardware::read(&node.kind, &node.data)
            .map(|p| p.parameters())
            .unwrap_or_default()
    }

    fn rebuild(&mut self, request: RebuildRequest) -> Vec<Rebuild> {
        request
            .bodies
            .into_iter()
            .map(|history| {
                let mut ops = Vec::new();
                let mut op_features = Vec::new();
                for (i, node) in history.features.iter().enumerate() {
                    let fail = |message: String| Rebuild {
                        body: history.body.clone(),
                        plan: Plan::Error {
                            feature: Some(node.id.clone()),
                            message,
                        },
                    };
                    let Some(part) = Hardware::read(&node.kind, &node.data) else {
                        return fail("The part's data does not read.".into());
                    };
                    if let Some(problem) = part.problem() {
                        return fail(problem);
                    }
                    if i > 0 {
                        return fail(
                            "A part stands on a body of its own; move this one to a new body."
                                .into(),
                        );
                    }
                    for op in part.ops() {
                        ops.push(op);
                        op_features.push(node.id.clone());
                    }
                }
                Rebuild {
                    body: history.body,
                    plan: if ops.is_empty() {
                        Plan::Empty
                    } else {
                        Plan::Ops { ops, op_features }
                    },
                }
            })
            .collect()
    }

    fn run_command(&mut self, command: &str, args: Value) -> Result<Value, String> {
        let suffix = command
            .strip_prefix(PACKAGE)
            .and_then(|s| s.strip_prefix('.'));
        match suffix {
            Some(CATALOG) => Ok(Self::catalog()),
            Some(MAKE) => {
                let name = args.get("part").and_then(Value::as_str).unwrap_or("screw");
                let family = Family::named(name).ok_or_else(|| {
                    format!(
                        "`{name}` is not a part: {}",
                        Family::ALL.map(|f| f.name()).join(", ")
                    )
                })?;
                let part = Hardware::with_args(family, &args, &self.defaults)?;
                if let Some(problem) = part.problem() {
                    return Err(problem);
                }
                let (body, feature) = self.make(&part)?;
                Ok(json!({"body": body, "feature": feature}))
            }
            _ => Err(format!("no command `{command}`")),
        }
    }

    fn input(&mut self, input: &Input) -> bool {
        match &input.event {
            Event::ToolActivated => {
                let tool = input.tool.as_deref().unwrap_or("");
                let tool = tool.split(':').next().unwrap_or(tool);
                let Some(def) = TOOLS.iter().find(|t| id(t.suffix) == tool) else {
                    return false;
                };
                let part = (def.seed)(&self.defaults);
                self.add(part, &format!("New {}", def.label.to_lowercase()));
                true
            }
            // A double click on a part's tree row.
            Event::EditFeature { feature } => {
                self.open(feature, false);
                true
            }
            Event::Key { key, down: true } if key == "Escape" && self.editing.is_some() => {
                self.task_close(false);
                true
            }
            _ => false,
        }
    }

    fn frame(&mut self, _pointer: &Pointer) -> Frame {
        let mut frame = Frame::default();
        let Some((id, part)) = self.edited() else {
            self.editing = None;
            return frame;
        };
        // The part's axis and name, where its body sits.
        let placement = host::feature(&id)
            .and_then(|n| n.body)
            .and_then(|b| host::bodies().into_iter().find(|x| x.id == b))
            .map(|b| b.placement);
        let world = |p: [f64; 3]| -> [f32; 3] {
            match placement {
                Some(m) => [
                    (m[0] * p[0] + m[1] * p[1] + m[2] * p[2] + m[3]) as f32,
                    (m[4] * p[0] + m[5] * p[1] + m[6] * p[2] + m[7]) as f32,
                    (m[8] * p[0] + m[9] * p[1] + m[10] * p[2] + m[11]) as f32,
                ],
                None => [p[0] as f32, p[1] as f32, p[2] as f32],
            }
        };
        let (from, to) = part.axis();
        let dir = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2])
            .sqrt()
            .max(1e-9);
        let over = |p: [f64; 3], k: f64| {
            [
                p[0] + dir[0] / len * k,
                p[1] + dir[1] / len * k,
                p[2] + dir[2] / len * k,
            ]
        };
        frame.lines.push(Polyline {
            points: vec![world(over(from, -2.0)), world(over(to, 2.0))],
            color: [0.31, 0.64, 0.9],
            width: 1.5,
            dashed: true,
            closed: false,
        });
        frame.labels.push(Label {
            at: world(over(to, 3.5)),
            text: part.label(),
            color: [0.9, 0.92, 0.94],
            size: 12.0,
            pill: true,
            mono: true,
        });
        frame.hud.tool = Some(ToolHint {
            icon: part.icon().into(),
            name: part.family().title().into(),
            prompt: "Set its standard and size in the panel".into(),
            keys: vec![("Esc".into(), "cancel".into())],
        });
        frame.status.selection = Some(part.label());
        frame.editing = Some(id.clone());
        frame.task = Some(Task {
            title: part.family().title().into(),
            icon: part.icon().into(),
            confirmable: true,
        });
        let ctx = Ctx {
            feature: &id,
            focus: self.focus.as_deref(),
        };
        frame.panel = part.panel(&ctx);
        frame.panel.push(Widget::Separator);
        frame.panel.push(Widget::Button {
            id: ANOTHER.into(),
            label: "Add another".into(),
            style: ButtonStyle::Secondary,
            enabled: part.problem().is_none(),
        });
        frame
    }

    fn panel_event(&mut self, slot: PanelSlot, event: PanelEvent) {
        if slot == PanelSlot::Settings {
            match event {
                PanelEvent::Choice { id, index } if id == "size" => {
                    if let Some(size) = standards::METRIC_SIZES.get(index) {
                        self.defaults.size = (*size).into();
                    }
                }
                PanelEvent::Choice { id, index } if id == "series" => {
                    if let Some(series) = standards::EXTRUSIONS.get(index) {
                        self.defaults.series = series.cell;
                    }
                }
                PanelEvent::Toggle { id, on } if id == "socket" => self.defaults.socket = on,
                PanelEvent::Toggle { id, on } if id == "thread" => self.defaults.thread = on,
                _ => {}
            }
            return;
        }
        let Some((id, mut part)) = self.edited() else {
            return;
        };
        if let PanelEvent::Button { id: button } = &event {
            if button == ANOTHER {
                self.add(part, "Add another part");
            }
            return;
        }
        if part.apply(&event) {
            self.focus = Some(event.id().to_string());
            self.write(&id, &part);
        }
    }

    fn task_close(&mut self, accept: bool) -> Option<String> {
        let (id, opened) = self.editing.take()?;
        self.focus = None;
        if accept {
            return Some(format!("Edit {}", opened.family().name()));
        }
        let undone = if self.fresh {
            // The tool made a body for it too; it goes with the part.
            let body = host::feature(&id).and_then(|n| n.body);
            host::remove_feature(&id).and_then(|()| match body {
                Some(body) => host::remove_body(&body),
                None => Ok(()),
            })
        } else {
            host::set_feature_data(&id, opened.to_value())
        };
        if let Err(e) = undone {
            host::error(&e);
        }
        None
    }

    fn menu_items(&mut self, scope: &MenuScope) -> Vec<MenuItem> {
        match scope {
            MenuScope::TreeFeature(feature)
                if host::feature(feature).is_some_and(|n| Family::from_kind(&n.kind).is_some()) =>
            {
                vec![MenuItem {
                    id: id(EDIT),
                    label: "Edit hardware".into(),
                    icon: Some("hardware".into()),
                    hint: None,
                    enabled: true,
                    separator_before: false,
                }]
            }
            _ => Vec::new(),
        }
    }

    fn menu_command(&mut self, command: &str, scope: &MenuScope) -> bool {
        match scope {
            MenuScope::TreeFeature(feature) if command == id(EDIT) => {
                self.open(feature, false);
                true
            }
            _ => false,
        }
    }

    fn settings_panel(&mut self) -> Vec<Widget> {
        let series: Vec<String> = standards::EXTRUSIONS
            .iter()
            .map(|s| format!("{} series", s.cell))
            .collect();
        vec![
            Widget::Heading {
                text: "New hardware".into(),
            },
            parts::choice(
                "size",
                "Thread size",
                &standards::METRIC_SIZES,
                parts::index_of(&standards::METRIC_SIZES, &self.defaults.size),
            ),
            parts::toggle(
                "socket",
                "Screws carry their drive recess",
                self.defaults.socket,
            ),
            parts::toggle(
                "thread",
                "Fasteners carry a modelled thread",
                self.defaults.thread,
            ),
            parts::choice(
                "series",
                "Extrusion series",
                &series,
                standards::EXTRUSIONS
                    .iter()
                    .position(|s| s.cell == self.defaults.series)
                    .unwrap_or(0),
            ),
        ]
    }

    fn settings(&self) -> Option<Value> {
        serde_json::to_value(&self.defaults).ok()
    }

    fn apply_settings(&mut self, settings: Value) {
        if let Ok(defaults) = serde_json::from_value(settings) {
            self.defaults = defaults;
        }
    }
}

bench!(HardwareBench);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_seeds_a_part_that_builds() {
        let defaults = Defaults::default();
        for tool in TOOLS {
            let part = (tool.seed)(&defaults);
            assert_eq!(part.problem(), None, "{}", tool.label);
            assert!(!part.ops().is_empty(), "{}", tool.label);
            assert_eq!(
                part.icon(),
                tool.icon,
                "{}: the tool's icon is the part's",
                tool.label
            );
        }
        let ids: std::collections::HashSet<_> = TOOLS.iter().map(|t| t.suffix).collect();
        assert_eq!(ids.len(), TOOLS.len(), "tool ids are distinct");
    }

    #[test]
    fn the_registration_names_everything_under_the_package() {
        let reg = HardwareBench::default().describe();
        assert_eq!(reg.tools.len(), TOOLS.len());
        for tool in &reg.tools {
            assert!(tool.id.starts_with(PACKAGE), "{}", tool.id);
            assert!(tool.category.is_some());
        }
        for command in &reg.commands {
            assert!(command.id.starts_with(PACKAGE));
        }
        assert!(reg.commands.iter().any(|c| c.read_only));
    }

    #[test]
    fn the_catalog_lists_each_family() {
        let catalog = HardwareBench::catalog();
        for family in Family::ALL {
            assert!(catalog.get(family.name()).is_some(), "{}", family.name());
        }
        assert_eq!(catalog["screw"][0]["head"], "socket_cap");
        assert_eq!(catalog["screw"][0]["standards"][0]["sizes"][2], "M3");
    }

    #[test]
    fn a_body_with_two_parts_is_refused_at_the_second() {
        let mut bench = HardwareBench::default();
        let part = seed_hex_nut(&Defaults::default());
        let node = |id: &str| Node {
            id: id.into(),
            kind: part.kind(),
            name: part.label(),
            body: Some("b".into()),
            data: part.to_value(),
            visible: true,
            made_by: None,
            deps: Vec::new(),
            seq: 0,
        };
        let plans = bench.rebuild(RebuildRequest {
            bodies: vec![BodyHistory {
                body: "b".into(),
                features: vec![node("one")],
            }],
        });
        assert!(
            matches!(&plans[0].plan, Plan::Ops { ops, op_features } if ops.len() == 2 && op_features.iter().all(|f| f == "one"))
        );
        let plans = bench.rebuild(RebuildRequest {
            bodies: vec![BodyHistory {
                body: "b".into(),
                features: vec![node("one"), node("two")],
            }],
        });
        assert!(matches!(&plans[0].plan, Plan::Error { feature: Some(f), .. } if f == "two"));
    }

    #[test]
    fn length_keys_cover_every_length_parameter() {
        let defaults = Defaults::default();
        for family in Family::ALL {
            let part = Hardware::with_args(family, &Value::Null, &defaults).unwrap();
            for p in part.parameters() {
                if p.dim == Dim::Length {
                    let key = p.pointer.trim_start_matches('/');
                    assert!(LENGTH_KEYS.contains(&key), "{}: {key}", family.name());
                }
            }
        }
    }
}
