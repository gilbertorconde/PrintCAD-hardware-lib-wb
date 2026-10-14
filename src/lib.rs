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
use parts::gear::{Gear, GearKind};
use parts::nut::NutKind;
use parts::rod::RodKind;
use parts::screw::Head;
use parts::tnut::TNutKind;
use parts::washer::WasherKind;
use parts::{
    Bearing, Ctx, Defaults, Extrusion, Family, Hardware, Insert, Magnet, Nut, PACKAGE, Rod, Screw,
    Spring, Standoff, TNut, Washer,
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
screw_tool!(seed_thumb, Head::Thumb);
nut_tool!(seed_hex_nut, NutKind::Hex);
nut_tool!(seed_thin_nut, NutKind::Thin);
nut_tool!(seed_nyloc_nut, NutKind::Nyloc);
nut_tool!(seed_square_nut, NutKind::Square);
nut_tool!(seed_wing_nut, NutKind::Wing);
nut_tool!(seed_thumb_nut, NutKind::Thumb);

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
macro_rules! gear_tool {
    ($name:ident, $kind:expr) => {
        fn $name(_: &Defaults) -> Hardware {
            Hardware::Gear(Gear::new($kind))
        }
    };
}

gear_tool!(seed_spur_gear, GearKind::Spur);
gear_tool!(seed_helical_gear, GearKind::Helical);
gear_tool!(seed_herringbone_gear, GearKind::Herringbone);
gear_tool!(seed_internal_gear, GearKind::Internal);
gear_tool!(seed_rack, GearKind::Rack);
gear_tool!(seed_worm, GearKind::Worm);
gear_tool!(seed_bevel_gear, GearKind::Bevel);
gear_tool!(seed_gt2_pulley, GearKind::Pulley);

fn seed_standoff(d: &Defaults) -> Hardware {
    Hardware::Standoff(Standoff::new(
        parts::standoff::StandoffShape::Hex,
        &d.size,
        d,
    ))
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
        suffix: "thumb",
        label: "Thumb screw",
        icon: "screw-thumb",
        category: "Screws",
        row: 2,
        seed: seed_thumb,
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
        suffix: "wing_nut",
        label: "Wing nut",
        icon: "nut-wing",
        category: "Nuts",
        row: 2,
        seed: seed_wing_nut,
    },
    ToolDef {
        suffix: "thumb_nut",
        label: "Thumb nut",
        icon: "nut-thumb",
        category: "Nuts",
        row: 2,
        seed: seed_thumb_nut,
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
    ToolDef {
        suffix: "standoff",
        label: "Standoff or spacer",
        icon: "standoff",
        category: "Other",
        row: 2,
        seed: seed_standoff,
    },
    ToolDef {
        suffix: "spur_gear",
        label: "Spur gear",
        icon: "gear-spur",
        category: "Gears",
        row: 1,
        seed: seed_spur_gear,
    },
    ToolDef {
        suffix: "helical_gear",
        label: "Helical gear",
        icon: "gear-helical",
        category: "Gears",
        row: 1,
        seed: seed_helical_gear,
    },
    ToolDef {
        suffix: "herringbone_gear",
        label: "Herringbone gear",
        icon: "gear-herringbone",
        category: "Gears",
        row: 1,
        seed: seed_herringbone_gear,
    },
    ToolDef {
        suffix: "bevel_gear",
        label: "Bevel gear",
        icon: "gear-bevel",
        category: "Gears",
        row: 1,
        seed: seed_bevel_gear,
    },
    ToolDef {
        suffix: "internal_gear",
        label: "Internal ring gear",
        icon: "gear-internal",
        category: "Gears",
        row: 2,
        seed: seed_internal_gear,
    },
    ToolDef {
        suffix: "rack",
        label: "Rack",
        icon: "gear-rack",
        category: "Gears",
        row: 2,
        seed: seed_rack,
    },
    ToolDef {
        suffix: "worm",
        label: "Worm",
        icon: "gear-worm",
        category: "Gears",
        row: 2,
        seed: seed_worm,
    },
    ToolDef {
        suffix: "gt2_pulley",
        label: "GT2 timing pulley",
        icon: "gear-pulley",
        category: "Gears",
        row: 2,
        seed: seed_gt2_pulley,
    },
];

fn id(suffix: &str) -> String {
    format!("{PACKAGE}.{suffix}")
}

/// Every numeric field of every family that is a length, for the
/// property panel to show in the document's unit.
const LENGTH_KEYS: [&str; 52] = [
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
    "e",
    "g",
    "across",
    "stud",
    "module",
    "thickness",
    "backlash",
    "flat",
    "key_w",
    "key_d",
    "hub",
    "hub_h",
    "rim",
    "worm_d",
    "pressure",
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
            "holes": standards::CLEARANCE_HOLES.iter().map(|h| {
                let (_, pitch) = standards::metric_any(h.size).unwrap_or((h.d, 0.0));
                let insert = standards::insert_for(h.d).map(|i| json!({"hole": i.hole, "depth": i.length + 1.0}));
                json!({
                    "size": h.size,
                    "clearance": {"fine": h.fine, "medium": h.medium, "coarse": h.coarse},
                    "tap_drill": ((h.d - pitch) * 100.0).round() / 100.0,
                    "insert": insert,
                })
            }).collect::<Vec<_>>(),
            "bearing": {
                "ball": standards::BALL_BEARINGS.iter().map(|r| r.name).collect::<Vec<_>>(),
                "linear": standards::LINEAR_BEARINGS.iter().map(|r| r.name).collect::<Vec<_>>(),
            },
            "magnet": {"shapes": ["disc", "ring", "block"]},
            "rod": RodKind::ALL.iter().map(|k| json!({"kind": k.tool(), "sizes": k.sizes()})).collect::<Vec<_>>(),
            "spring": {"fields": ["outer", "wire", "length", "turns"]},
            "gear": {
                "kinds": GearKind::ALL.iter().map(|k| k.tool()).collect::<Vec<_>>(),
                "shafts": parts::gear::Shaft::ALL.iter().map(|s| s.tool()).collect::<Vec<_>>(),
                "presets": parts::gear::SHAFTS.iter().filter(|s| s.1 > 0.0).map(|s| json!({"name": s.0, "bore": s.1, "shaft": s.2.tool(), "flat": s.3})).collect::<Vec<_>>(),
                "fields": ["teeth", "module", "pressure", "thickness", "backlash", "helix", "hand", "mate", "bore", "shaft", "flat", "key_w", "key_d", "hub", "hub_h", "set_screw", "length", "rim", "worm_d", "starts", "flanges"],
            },
            "standoff": {
                "shapes": ["hex", "round"],
                "sizes": standards::STANDOFFS.iter().map(|r| r.size).collect::<Vec<_>>(),
                "ends": ["ff", "mf"],
                "bores": ["thread", "clear", "insert"],
            },
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
            description: "Standard hardware sized from its standards: screws, nuts, washers, T-slot extrusions and nuts, inserts, standoffs, bearings, magnets, rods, springs and gears".into(),
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
                        param("part", ParamKind::String, true, "`screw`, `nut`, `tnut`, `washer`, `extrusion`, `insert`, `bearing`, `magnet`, `rod`, `spring`, `standoff` or `gear`"),
                        param("head", ParamKind::String, false, "a screw's head: `socket_cap` (the default), `button`, `countersunk`, `hex_bolt`, `low_head`, `set_screw` or `thumb`"),
                        param("kind", ParamKind::String, false, "a nut's (`hex`, `thin`, `nyloc`, `square`, `wing`, `thumb`), T-slot nut's (`sliding`, `drop_in`, `spring_ball`, `twist`, `roll_in`), washer's (`flat`, `large`, `spring`), bearing's (`ball`, `linear`), rod's (`shaft`, `threaded_rod`, `lead_screw`, `dowel_pin`) or gear's (`spur`, `helical`, `herringbone`, `internal`, `rack`, `worm`, `bevel`, `gt2_pulley`) kind"),
                        param("teeth", ParamKind::Integer, false, "a gear's teeth; `module`, `pressure`, `thickness`, `backlash`, `helix`, `hand`, `mate`, `bore`, `shaft` (`round`, `d`, `dd`, `hex`, `keyed`), `flat`, `hub`, `hub_h`, `set_screw`, `length`, `rim`, `worm_d`, `starts` and `flanges` as the panel has them"),
                        param("standard", ParamKind::String, false, "the standard to size by, as `catalog` names it; the first when left out"),
                        param("size", ParamKind::String, false, "the thread size, `M3`; the Preferences default when left out; a size off the standard's table (`M7`) is made in proportion to its thread, as is a `d` given alone"),
                        param("length", ParamKind::Number, false, "mm; a screw's from under its head"),
                        param("thread", ParamKind::Bool, false, "model the thread"),
                        param("socket", ParamKind::Bool, false, "model a screw's drive recess"),
                        param("series", ParamKind::Integer, false, "an extrusion's or T-nut's series: 20, 30 or 40"),
                        param("profile", ParamKind::String, false, "an extrusion's profile, `2040`"),
                        param("along", ParamKind::String, false, "an extrusion's axis: `X`, `Y` or `Z`"),
                        param("slots", ParamKind::String, false, "an extrusion's slotted faces: `all`, `three`, `adjacent`, `opposite` or `one`"),
                        param("name", ParamKind::String, false, "a bearing's designation, `608` or `LM8UU`"),
                        param("shape", ParamKind::String, false, "a magnet's shape: `disc`, `ring` or `block`; a standoff's: `hex` or `round`"),
                        param("ends", ParamKind::String, false, "a standoff's ends: `ff` (female at both) or `mf` (a stud on top)"),
                        param("bore", ParamKind::String, false, "a standoff's bore: `thread`, `clear` (a spacer) or `insert` (heat-set insert holes)"),
                        param("stud", ParamKind::Number, false, "a male-female standoff's stud length, mm"),
                        param("d", ParamKind::Number, false, "any dimension the part's panel shows may be given by its name (`d`, `dk`, `k`, `s`, `m`, `outer`, `wire`, `turns`…) and makes the part custom"),
                    ],
                    returns: "{body, feature}".into(),
                    read_only: false,
                    ..Default::default()
                },
                Command {
                    id: id(CATALOG),
                    summary: "The kinds, standards and sizes the package offers".into(),
                    params: Vec::new(),
                    returns: "{screw, nut, tnut, washer, extrusion, insert, holes, bearing, magnet, rod, spring, standoff, gear}".into(),
                    read_only: true,
                    ..Default::default()
                },
            ],
            length_keys: LENGTH_KEYS.map(String::from).to_vec(),
            bought_kinds: Family::ALL
                .into_iter()
                .filter(|f| f.bought())
                .map(Family::kind)
                .collect(),
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
                let mut threads = Vec::new();
                for (i, node) in history.features.iter().enumerate() {
                    let fail = |message: String| Rebuild {
                        body: history.body.clone(),
                        plan: Plan::Error {
                            feature: Some(node.id.clone()),
                            message,
                        },
                        threads: Vec::new(),
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
                    threads.extend(part.threads().into_iter().map(|t| CosmeticThread {
                        feature: node.id.clone(),
                        point: t.point,
                        pitch: t.pitch,
                        length: t.length,
                        designation: Some(t.designation),
                        ..Default::default()
                    }));
                }
                Rebuild {
                    body: history.body,
                    plan: if ops.is_empty() {
                        Plan::Empty
                    } else {
                        Plan::Ops { ops, op_features }
                    },
                    threads,
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

    /// Catalog hardware is declared bought, so the parts list marks it
    /// and exports leave it out; gears, often printed, are not.
    #[test]
    fn hardware_but_gears_is_declared_bought() {
        let bought = HardwareBench::default().describe().bought_kinds;
        assert!(bought.contains(&Family::Screw.kind()));
        assert!(bought.contains(&Family::Bearing.kind()));
        assert!(!bought.contains(&Family::Gear.kind()));
        assert_eq!(bought.len(), Family::ALL.len() - 1);
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

    /// The plan for one part on its own body, made by `family`'s command
    /// with `args`.
    fn plan_of(family: Family, args: Value) -> Rebuild {
        let part = Hardware::with_args(family, &args, &Defaults::default()).unwrap();
        let node = Node {
            id: "f".into(),
            kind: part.kind(),
            name: part.label(),
            body: Some("b".into()),
            data: part.to_value(),
            visible: true,
            made_by: None,
            deps: Vec::new(),
            seq: 0,
        };
        let mut plans = HardwareBench::default().rebuild(RebuildRequest {
            bodies: vec![BodyHistory {
                body: "b".into(),
                features: vec![node],
            }],
        });
        assert!(matches!(plans[0].plan, Plan::Ops { .. }));
        plans.remove(0)
    }

    fn radius(point: [f64; 3]) -> f64 {
        point[0].hypot(point[1])
    }

    /// A screw's thread is drawn outside its shank, from above the tip's
    /// chamfer to where the standard's thread length ends.
    #[test]
    fn a_screw_declares_its_thread_on_the_shank() {
        let plan = plan_of(Family::Screw, json!({"size": "M6", "length": 30}));
        let [thread] = plan.threads.as_slice() else {
            panic!("one thread: {:?}", plan.threads);
        };
        assert_eq!(thread.feature, "f");
        assert_eq!(thread.designation.as_deref(), Some("M6"));
        assert_eq!(thread.pitch, 1.0);
        assert!(!thread.left_handed);
        assert!((radius(thread.point) - 3.0).abs() < 1e-9, "on the shank");
        let tip = 0.6;
        assert!(
            (thread.point[2] - (-30.0 + tip + 1.0)).abs() < 1e-9,
            "a pitch up the face"
        );
        // b = 2d + 12 from the tip, the face starting above the chamfer.
        assert!((-30.0 + tip + thread.length - (-30.0 + 24.0)).abs() < 1e-9);

        let fully = plan_of(Family::Screw, json!({"head": "set_screw", "size": "M4"}));
        assert_eq!(fully.threads[0].length, 0.0, "a set screw's whole shank");
    }

    /// A nut's thread is drawn inside its bore, at the tap drill, the
    /// whole height; a lock nut's stops under its collar.
    #[test]
    fn a_nut_declares_its_thread_in_the_bore() {
        let plan = plan_of(Family::Nut, json!({"size": "M5"}));
        let [thread] = plan.threads.as_slice() else {
            panic!("one thread: {:?}", plan.threads);
        };
        assert_eq!(thread.designation.as_deref(), Some("M5"));
        assert_eq!(thread.pitch, 0.8);
        assert_eq!(thread.length, 0.0);
        let minor = standards::internal_minor(5.0, 0.8);
        assert!(
            (radius(thread.point) - minor / 2.0).abs() < 1e-9,
            "in the bore"
        );
        assert!(thread.point[2] > 0.0 && thread.point[2] <= 0.8);

        let lock = plan_of(Family::Nut, json!({"kind": "nyloc", "size": "M5"}));
        let Hardware::Nut(nut) = Hardware::with_args(
            Family::Nut,
            &json!({"kind": "nyloc", "size": "M5"}),
            &Defaults::default(),
        )
        .unwrap() else {
            unreachable!()
        };
        assert!(lock.threads[0].length > 0.0 && lock.threads[0].length < nut.m);
    }

    /// A thread cut into the part is not drawn over as well.
    #[test]
    fn a_modelled_thread_is_not_declared() {
        for family in [Family::Screw, Family::Nut, Family::Insert] {
            let plan = plan_of(family, json!({"size": "M4", "thread": true}));
            assert!(plan.threads.is_empty(), "{}", family.name());
        }
    }

    /// Each thread is called as its standard writes it: a fine pitch
    /// after the size, an inch screw by its count and series, a lead
    /// screw by its lead and pitch, drawn a turn to its lead.
    #[test]
    fn threads_carry_their_designations() {
        let designation = |family, args| {
            let plan = plan_of(family, args);
            plan.threads
                .iter()
                .map(|t| (t.designation.clone().unwrap(), t.pitch))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            designation(Family::Screw, json!({"size": "M8", "pitch": 1.0})),
            [("M8x1".into(), 1.0)]
        );
        assert_eq!(
            designation(Family::Screw, json!({"standard": "ASME", "size": "1/4-20"})),
            [("1/4-20 UNC".into(), 25.4 / 20.0)]
        );
        assert_eq!(
            designation(
                Family::Rod,
                json!({"kind": "lead_screw", "size": "T8 (lead 8)"})
            ),
            [("Tr8x8 (P2)".into(), 8.0)]
        );
        assert_eq!(
            designation(Family::Rod, json!({"kind": "threaded_rod", "size": "M10"})),
            [("M10".into(), 1.5)]
        );
        assert_eq!(
            designation(Family::Insert, json!({"size": "M3"})),
            [("M3".into(), 0.5)]
        );
        assert_eq!(
            designation(Family::TNut, json!({"kind": "roll_in", "size": "M5"})),
            [("M5".into(), 0.8), ("M4".into(), 0.7)],
            "a roll-in nut's set screw hole is tapped M4"
        );
    }

    /// A male-female standoff is threaded twice: its bore below, at the
    /// tap drill, and its stud above, at the thread's diameter.
    #[test]
    fn a_male_female_standoff_declares_its_bore_and_its_stud() {
        let plan = plan_of(
            Family::Standoff,
            json!({"size": "M3", "length": 10, "stud": 6}),
        );
        let [bore, stud] = plan.threads.as_slice() else {
            panic!("two threads: {:?}", plan.threads);
        };
        assert!((radius(bore.point) - standards::internal_minor(3.0, 0.5) / 2.0).abs() < 1e-9);
        assert!(bore.point[2] < 10.0);
        assert!((radius(stud.point) - 1.5).abs() < 1e-9);
        assert!(stud.point[2] > 10.0 && stud.point[2] < 16.0);
        assert_eq!(stud.designation.as_deref(), Some("M3"));
    }

    /// Every tool's part that has a thread declares it, at a point within
    /// the part's run along its axis; the rest declare none.
    #[test]
    fn every_threaded_tool_declares_its_threads() {
        let defaults = Defaults::default();
        for tool in TOOLS {
            let part = (tool.seed)(&defaults);
            let threads = part.threads();
            let threaded = match &part {
                Hardware::Screw(_)
                | Hardware::Nut(_)
                | Hardware::TNut(_)
                | Hardware::Insert(_)
                | Hardware::Standoff(_) => true,
                Hardware::Rod(r) => r.kind == RodKind::Threaded,
                Hardware::Gear(g) => g.set_screw > 0.0,
                _ => false,
            };
            assert_eq!(!threads.is_empty(), threaded, "{}", tool.label);
            let (from, to) = part.axis();
            for t in threads {
                assert!(t.pitch > 0.0 && t.length >= 0.0, "{}", tool.label);
                assert!(t.designation.starts_with('M'), "{}", tool.label);
                if from[0] == 0.0 && to[0] == 0.0 {
                    let (low, high) = (from[2].min(to[2]), from[2].max(to[2]));
                    assert!(
                        t.point[2] > low && t.point[2] < high,
                        "{}: {:?} within {low}..{high}",
                        tool.label,
                        t.point
                    );
                }
            }
        }
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
