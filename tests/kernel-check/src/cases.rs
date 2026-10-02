//! Every part the checks build: one of each kind and size that matters,
//! with the modelled-thread and odd-shape variants that have failed before.

use crate::parts::bearing::BearingKind;
use crate::parts::extrusion::Slots;
use crate::parts::magnet::MagnetShape;
use crate::parts::nut::NutKind;
use crate::parts::rod::RodKind;
use crate::parts::screw::Head;
use crate::parts::tnut::TNutKind;
use crate::parts::washer::WasherKind;
use crate::parts::{
    Bearing, Defaults, Extrusion, Hardware, Insert, Magnet, Nut, Rod, Screw, Spring, TNut, Washer,
};

pub struct Case {
    pub name: String,
    pub part: Hardware,
}

pub fn cases() -> Vec<Case> {
    let d = Defaults::default();
    let mut out = Vec::new();
    let mut case = |name: &str, part: Hardware| {
        out.push(Case {
            name: name.into(),
            part,
        })
    };
    for head in Head::ALL {
        let screw = Screw::new(head, "M4", &d);
        case(
            &format!("screw {} M4", head.tool()),
            Hardware::Screw(screw.clone()),
        );
        let mut plain = screw.clone();
        plain.socket = false;
        case(
            &format!("screw {} M4, no recess", head.tool()),
            Hardware::Screw(plain),
        );
    }
    let mut threaded = Screw::new(Head::SocketCap, "M3", &d);
    threaded.thread = true;
    threaded.length = 8.0;
    case(
        "screw socket_cap M3 × 8, modelled thread",
        Hardware::Screw(threaded),
    );
    let mut inch = Screw::new(Head::SocketCap, "M6", &d);
    inch.standard = "ASME B18.3 (inch)".into();
    inch.size = "1/4-20".into();
    inch.refill();
    case("screw socket_cap 1/4-20", Hardware::Screw(inch));
    let mut set = Screw::new(Head::Set, "M3", &d);
    set.thread = true;
    set.length = 6.0;
    case(
        "screw set_screw M3 × 6, modelled thread",
        Hardware::Screw(set),
    );
    for kind in NutKind::ALL {
        case(
            &format!("nut {} M4", kind.tool()),
            Hardware::Nut(Nut::new(kind, "M4", &d)),
        );
    }
    let mut threaded = Nut::new(NutKind::Hex, "M3", &d);
    threaded.thread = true;
    case("nut hex M3, modelled thread", Hardware::Nut(threaded));
    for kind in WasherKind::ALL {
        case(
            &format!("washer {} M4", kind.tool()),
            Hardware::Washer(Washer::new(kind, "M4")),
        );
    }
    case(
        "extrusion 2020 × 40",
        Hardware::Extrusion(Extrusion::new(20, 1, 1, 40.0)),
    );
    case(
        "extrusion 2040 × 40",
        Hardware::Extrusion(Extrusion::new(20, 1, 2, 40.0)),
    );
    let mut v = Extrusion::new(20, 1, 1, 40.0);
    v.v_slot = true;
    v.refill();
    case(
        "extrusion 2040 × 40, V-slot",
        Hardware::Extrusion({
            let mut v = v.clone();
            v.cells_y = 2;
            v
        }),
    );
    case("extrusion 2020 × 40, V-slot", Hardware::Extrusion(v));
    let mut x = Extrusion::new(40, 1, 1, 60.0);
    x.along = "X".into();
    case("extrusion 4040 × 60 along X", Hardware::Extrusion(x));
    case(
        "extrusion 3060 × 30",
        Hardware::Extrusion(Extrusion::new(30, 1, 2, 30.0)),
    );
    for slots in [Slots::Three, Slots::Adjacent, Slots::Opposite, Slots::One] {
        let mut e = Extrusion::new(20, 1, 1, 30.0);
        e.slots = slots;
        case(
            &format!("extrusion 2020 × 30, {:?}", slots),
            Hardware::Extrusion(e),
        );
    }
    let mut f = Extrusion::new(20, 1, 2, 30.0);
    f.slots = Slots::Three;
    case("extrusion 2040 × 30, Three", Hardware::Extrusion(f));
    case(
        "extrusion 3030 × 30",
        Hardware::Extrusion(Extrusion::new(30, 1, 1, 30.0)),
    );
    case(
        "extrusion 4040 × 30",
        Hardware::Extrusion(Extrusion::new(40, 1, 1, 30.0)),
    );
    case(
        "extrusion 2080 × 30",
        Hardware::Extrusion(Extrusion::new(20, 1, 4, 30.0)),
    );
    for series in [20, 30, 40] {
        for kind in TNutKind::ALL {
            case(
                &format!("tnut {:?} {series}", kind),
                Hardware::TNut(TNut::new(kind, series, "M5", &d)),
            );
        }
    }
    let mut threaded = TNut::new(TNutKind::DropIn, 20, "M5", &d);
    threaded.thread = true;
    case(
        "tnut DropIn 20 M5, modelled thread",
        Hardware::TNut(threaded),
    );
    case("insert M3", Hardware::Insert(Insert::new("M3", &d)));
    let mut insert = Insert::new("M4", &d);
    insert.thread = true;
    case("insert M4, modelled thread", Hardware::Insert(insert));
    case(
        "bearing 608",
        Hardware::Bearing(Bearing::new(BearingKind::Ball, "608")),
    );
    case(
        "bearing MR85",
        Hardware::Bearing(Bearing::new(BearingKind::Ball, "MR85")),
    );
    case(
        "bearing LM8UU",
        Hardware::Bearing(Bearing::new(BearingKind::Linear, "LM8UU")),
    );
    for shape in MagnetShape::ALL {
        case(
            &format!("magnet {}", shape.name()),
            Hardware::Magnet(Magnet::new(shape)),
        );
    }
    for kind in RodKind::ALL {
        let mut rod = Rod::new(kind, "Ø8", &d);
        rod.length = 30.0;
        case(&format!("rod {} × 30", kind.tool()), Hardware::Rod(rod));
    }
    let mut rod = Rod::new(RodKind::Threaded, "M6", &d);
    rod.length = 12.0;
    rod.thread = true;
    case(
        "rod threaded_rod M6 × 12, modelled thread",
        Hardware::Rod(rod),
    );
    for (size, length) in [("M3", 12.0), ("M4", 12.0), ("M5", 12.0), ("M8", 12.0)] {
        let mut rod = Rod::new(RodKind::Threaded, size, &d);
        rod.length = length;
        rod.thread = true;
        case(
            &format!("rod threaded_rod {size} × {length}, modelled thread"),
            Hardware::Rod(rod),
        );
    }
    for (size, length) in [("M4", 10.0), ("M5", 12.0), ("M6", 16.0)] {
        let mut screw = Screw::new(Head::SocketCap, size, &d);
        screw.length = length;
        screw.thread = true;
        case(
            &format!("screw socket_cap {size} × {length}, modelled thread"),
            Hardware::Screw(screw),
        );
    }
    for (size, length) in [("M3", 8.0), ("M4", 10.0)] {
        let mut screw = Screw::new(Head::SocketCap, size, &d);
        screw.length = length;
        screw.thread = true;
        screw.socket = false;
        case(
            &format!("screw socket_cap {size} × {length}, modelled thread, no recess"),
            Hardware::Screw(screw),
        );
        let mut hex = Screw::new(Head::Hex, size, &d);
        hex.length = length;
        hex.thread = true;
        case(
            &format!("screw hex_bolt {size} × {length}, modelled thread"),
            Hardware::Screw(hex),
        );
    }
    let mut long = Screw::new(Head::SocketCap, "M3", &d);
    long.length = 20.0;
    long.thread = true;
    case(
        "screw socket_cap M3 × 20, modelled thread, lead-in",
        Hardware::Screw(long),
    );
    case("spring Ø10 × 25", Hardware::Spring(Spring::default()));
    use crate::parts::standoff::{Bore, Standoff, StandoffShape};
    case(
        "standoff hex M3 × 10",
        Hardware::Standoff(Standoff::new(StandoffShape::Hex, "M3", &d)),
    );
    let mut mf = Standoff::new(StandoffShape::Hex, "M3", &d);
    mf.stud = 6.0;
    mf.thread = true;
    case("standoff hex M3 × 10 male-female, modelled thread", Hardware::Standoff(mf));
    let mut ins = Standoff::new(StandoffShape::Round, "M4", &d);
    ins.bore = Bore::Insert;
    ins.length = 25.0;
    case("standoff round M4 × 25, insert holes", Hardware::Standoff(ins));
    let mut spacer = Standoff::new(StandoffShape::Round, "M5", &d);
    spacer.bore = Bore::Clear;
    case("spacer round M5 × 15", Hardware::Standoff(spacer));
    use crate::parts::gear::{Gear, GearKind, Shaft};
    for kind in GearKind::ALL {
        case(&format!("gear {}", kind.tool()), Hardware::Gear(Gear::new(kind)));
    }
    let mut hubbed = Gear::new(GearKind::Spur);
    hubbed.hub = 14.0;
    hubbed.set_screw = 3.0;
    hubbed.shaft = Shaft::Keyed;
    case("gear spur, keyed, hub and set screw", Hardware::Gear(hubbed));
    let mut hexed = Gear::new(GearKind::Helical);
    hexed.shaft = Shaft::Hex;
    hexed.bore = 6.0;
    hexed.left = true;
    hexed.helix = 30.0;
    case("gear helical 30° left, hex bore", Hardware::Gear(hexed));
    let mut worm = Gear::new(GearKind::Worm);
    worm.starts = 2;
    case("gear worm 2-start", Hardware::Gear(worm));
    let mut big = Gear::new(GearKind::Internal);
    big.teeth = 40;
    case("gear internal 40T", Hardware::Gear(big));
    out
}
