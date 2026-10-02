//! The tables the parts are sized from: thread pitches, head and nut
//! dimensions by standard, washer rings, extrusion series, inserts,
//! bearings and the rest. Lengths in millimetres.
//!
//! Each table is a list of rows in the order the panel offers them. The
//! dimensions are the standards' nominal (maximum) values; where a
//! standard gives a range, the common manufactured size is used.

/// ISO 261 coarse pitches by nominal diameter.
pub const METRIC_COARSE: [(&str, f64, f64); 13] = [
    ("M1.6", 1.6, 0.35),
    ("M2", 2.0, 0.4),
    ("M2.5", 2.5, 0.45),
    ("M3", 3.0, 0.5),
    ("M4", 4.0, 0.7),
    ("M5", 5.0, 0.8),
    ("M6", 6.0, 1.0),
    ("M8", 8.0, 1.25),
    ("M10", 10.0, 1.5),
    ("M12", 12.0, 1.75),
    ("M16", 16.0, 2.0),
    ("M20", 20.0, 2.5),
    ("M24", 24.0, 3.0),
];

/// The diameter and coarse pitch of a metric size, `M3` → `(3.0, 0.5)`.
pub fn metric(size: &str) -> Option<(f64, f64)> {
    METRIC_COARSE
        .iter()
        .find(|(name, _, _)| name.eq_ignore_ascii_case(size))
        .map(|(_, d, p)| (*d, *p))
}

/// The metric sizes the thread tables offer, `M2` to `M12`.
pub const METRIC_SIZES: [&str; 9] = ["M2", "M2.5", "M3", "M4", "M5", "M6", "M8", "M10", "M12"];

/// A screw size of one standard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrewRow {
    pub size: &'static str,
    /// Thread diameter and pitch.
    pub d: f64,
    pub pitch: f64,
    /// Head diameter; a hex head's width across flats; 0 for no head.
    pub dk: f64,
    /// Head height; 0 for no head.
    pub k: f64,
    /// The drive's hex key across flats; 0 for an external hex.
    pub s: f64,
    /// The drive's depth.
    pub t: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrewTable {
    pub name: &'static str,
    /// In inches as made, so lengths in the panel are offered in inches.
    pub inch: bool,
    pub rows: &'static [ScrewRow],
}

const fn row(size: &'static str, d: f64, pitch: f64, dk: f64, k: f64, s: f64, t: f64) -> ScrewRow {
    ScrewRow {
        size,
        d,
        pitch,
        dk,
        k,
        s,
        t,
    }
}

/// Socket head cap screws.
pub const ISO_4762: ScrewTable = ScrewTable {
    name: "ISO 4762 / DIN 912",
    inch: false,
    rows: &[
        row("M2", 2.0, 0.4, 3.8, 2.0, 1.5, 1.0),
        row("M2.5", 2.5, 0.45, 4.5, 2.5, 2.0, 1.1),
        row("M3", 3.0, 0.5, 5.5, 3.0, 2.5, 1.3),
        row("M4", 4.0, 0.7, 7.0, 4.0, 3.0, 2.0),
        row("M5", 5.0, 0.8, 8.5, 5.0, 4.0, 2.5),
        row("M6", 6.0, 1.0, 10.0, 6.0, 5.0, 3.0),
        row("M8", 8.0, 1.25, 13.0, 8.0, 6.0, 4.0),
        row("M10", 10.0, 1.5, 16.0, 10.0, 8.0, 5.0),
        row("M12", 12.0, 1.75, 18.0, 12.0, 10.0, 6.0),
        row("M16", 16.0, 2.0, 24.0, 16.0, 14.0, 8.0),
        row("M20", 20.0, 2.5, 30.0, 20.0, 17.0, 10.0),
    ],
};

const IN: f64 = 25.4;

/// Socket head cap screws, inch series.
pub const ASME_B18_3: ScrewTable = ScrewTable {
    name: "ASME B18.3 (inch)",
    inch: true,
    rows: &[
        row(
            "#4-40",
            0.112 * IN,
            IN / 40.0,
            0.183 * IN,
            0.112 * IN,
            0.09375 * IN,
            0.051 * IN,
        ),
        row(
            "#6-32",
            0.138 * IN,
            IN / 32.0,
            0.226 * IN,
            0.138 * IN,
            0.109375 * IN,
            0.064 * IN,
        ),
        row(
            "#8-32",
            0.164 * IN,
            IN / 32.0,
            0.270 * IN,
            0.164 * IN,
            0.140625 * IN,
            0.077 * IN,
        ),
        row(
            "#10-24",
            0.190 * IN,
            IN / 24.0,
            0.312 * IN,
            0.190 * IN,
            0.15625 * IN,
            0.090 * IN,
        ),
        row(
            "1/4-20",
            0.250 * IN,
            IN / 20.0,
            0.375 * IN,
            0.250 * IN,
            0.1875 * IN,
            0.120 * IN,
        ),
        row(
            "5/16-18",
            0.3125 * IN,
            IN / 18.0,
            0.469 * IN,
            0.3125 * IN,
            0.25 * IN,
            0.151 * IN,
        ),
        row(
            "3/8-16",
            0.375 * IN,
            IN / 16.0,
            0.562 * IN,
            0.375 * IN,
            0.3125 * IN,
            0.182 * IN,
        ),
        row(
            "1/2-13",
            0.5 * IN,
            IN / 13.0,
            0.750 * IN,
            0.5 * IN,
            0.375 * IN,
            0.245 * IN,
        ),
    ],
};

/// Button head socket screws.
pub const ISO_7380: ScrewTable = ScrewTable {
    name: "ISO 7380-1",
    inch: false,
    rows: &[
        row("M2.5", 2.5, 0.45, 4.7, 1.5, 1.5, 0.8),
        row("M3", 3.0, 0.5, 5.7, 1.65, 2.0, 1.04),
        row("M4", 4.0, 0.7, 7.6, 2.2, 2.5, 1.3),
        row("M5", 5.0, 0.8, 9.5, 2.75, 3.0, 1.56),
        row("M6", 6.0, 1.0, 10.5, 3.3, 4.0, 2.08),
        row("M8", 8.0, 1.25, 14.0, 4.4, 5.0, 2.6),
        row("M10", 10.0, 1.5, 17.5, 5.5, 6.0, 3.12),
        row("M12", 12.0, 1.75, 21.0, 6.6, 8.0, 4.16),
        row("M16", 16.0, 2.0, 28.0, 8.8, 10.0, 5.2),
    ],
};

/// Countersunk socket screws, 90° heads.
pub const ISO_10642: ScrewTable = ScrewTable {
    name: "ISO 10642 / DIN 7991",
    inch: false,
    rows: &[
        row("M3", 3.0, 0.5, 6.0, 1.7, 2.0, 1.2),
        row("M4", 4.0, 0.7, 8.0, 2.3, 2.5, 1.8),
        row("M5", 5.0, 0.8, 10.0, 2.8, 3.0, 2.3),
        row("M6", 6.0, 1.0, 12.0, 3.3, 4.0, 2.5),
        row("M8", 8.0, 1.25, 16.0, 4.4, 5.0, 3.5),
        row("M10", 10.0, 1.5, 20.0, 5.5, 6.0, 3.6),
        row("M12", 12.0, 1.75, 24.0, 6.5, 8.0, 4.3),
        row("M16", 16.0, 2.0, 30.0, 7.5, 10.0, 5.3),
        row("M20", 20.0, 2.5, 36.0, 8.5, 12.0, 6.4),
    ],
};

/// Hex head screws, threaded to the head. `dk` is the width across flats.
pub const ISO_4017: ScrewTable = ScrewTable {
    name: "ISO 4017 / DIN 933",
    inch: false,
    rows: &[
        row("M3", 3.0, 0.5, 5.5, 2.0, 0.0, 0.0),
        row("M4", 4.0, 0.7, 7.0, 2.8, 0.0, 0.0),
        row("M5", 5.0, 0.8, 8.0, 3.5, 0.0, 0.0),
        row("M6", 6.0, 1.0, 10.0, 4.0, 0.0, 0.0),
        row("M8", 8.0, 1.25, 13.0, 5.3, 0.0, 0.0),
        row("M10", 10.0, 1.5, 16.0, 6.4, 0.0, 0.0),
        row("M12", 12.0, 1.75, 18.0, 7.5, 0.0, 0.0),
        row("M16", 16.0, 2.0, 24.0, 10.0, 0.0, 0.0),
        row("M20", 20.0, 2.5, 30.0, 12.5, 0.0, 0.0),
    ],
};

/// Low head socket cap screws.
pub const DIN_7984: ScrewTable = ScrewTable {
    name: "DIN 7984",
    inch: false,
    rows: &[
        row("M3", 3.0, 0.5, 5.5, 2.0, 2.0, 1.0),
        row("M4", 4.0, 0.7, 7.0, 2.8, 2.5, 1.4),
        row("M5", 5.0, 0.8, 8.5, 3.5, 3.0, 1.7),
        row("M6", 6.0, 1.0, 10.0, 4.0, 4.0, 2.0),
        row("M8", 8.0, 1.25, 13.0, 5.0, 5.0, 2.5),
        row("M10", 10.0, 1.5, 16.0, 6.5, 7.0, 3.0),
        row("M12", 12.0, 1.75, 18.0, 7.5, 8.0, 3.5),
        row("M16", 16.0, 2.0, 24.0, 10.0, 12.0, 4.5),
    ],
};

/// Hex socket set screws with a flat point. `dk` is the point's diameter.
pub const ISO_4026: ScrewTable = ScrewTable {
    name: "ISO 4026 / DIN 913",
    inch: false,
    rows: &[
        row("M2", 2.0, 0.4, 1.0, 0.0, 0.9, 0.8),
        row("M2.5", 2.5, 0.45, 1.5, 0.0, 1.3, 1.2),
        row("M3", 3.0, 0.5, 2.0, 0.0, 1.5, 1.2),
        row("M4", 4.0, 0.7, 2.5, 0.0, 2.0, 1.5),
        row("M5", 5.0, 0.8, 3.5, 0.0, 2.5, 2.0),
        row("M6", 6.0, 1.0, 4.0, 0.0, 3.0, 2.0),
        row("M8", 8.0, 1.25, 5.5, 0.0, 4.0, 3.0),
        row("M10", 10.0, 1.5, 7.0, 0.0, 5.0, 4.0),
        row("M12", 12.0, 1.75, 8.5, 0.0, 6.0, 4.8),
    ],
};

/// A nut size of one standard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NutRow {
    pub size: &'static str,
    pub d: f64,
    pub pitch: f64,
    /// Width across flats (a square nut's side).
    pub s: f64,
    /// Height over all.
    pub m: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NutTable {
    pub name: &'static str,
    pub rows: &'static [NutRow],
}

const fn nut(size: &'static str, d: f64, pitch: f64, s: f64, m: f64) -> NutRow {
    NutRow {
        size,
        d,
        pitch,
        s,
        m,
    }
}

pub const ISO_4032: NutTable = NutTable {
    name: "ISO 4032 / DIN 934",
    rows: &[
        nut("M2", 2.0, 0.4, 4.0, 1.6),
        nut("M2.5", 2.5, 0.45, 5.0, 2.0),
        nut("M3", 3.0, 0.5, 5.5, 2.4),
        nut("M4", 4.0, 0.7, 7.0, 3.2),
        nut("M5", 5.0, 0.8, 8.0, 4.7),
        nut("M6", 6.0, 1.0, 10.0, 5.2),
        nut("M8", 8.0, 1.25, 13.0, 6.8),
        nut("M10", 10.0, 1.5, 16.0, 8.4),
        nut("M12", 12.0, 1.75, 18.0, 10.8),
        nut("M16", 16.0, 2.0, 24.0, 14.8),
        nut("M20", 20.0, 2.5, 30.0, 18.0),
    ],
};

/// Thin (jam) hex nuts.
pub const ISO_4035: NutTable = NutTable {
    name: "ISO 4035 / DIN 439",
    rows: &[
        nut("M3", 3.0, 0.5, 5.5, 1.8),
        nut("M4", 4.0, 0.7, 7.0, 2.2),
        nut("M5", 5.0, 0.8, 8.0, 2.7),
        nut("M6", 6.0, 1.0, 10.0, 3.2),
        nut("M8", 8.0, 1.25, 13.0, 4.0),
        nut("M10", 10.0, 1.5, 16.0, 5.0),
        nut("M12", 12.0, 1.75, 18.0, 6.0),
        nut("M16", 16.0, 2.0, 24.0, 8.0),
        nut("M20", 20.0, 2.5, 30.0, 10.0),
    ],
};

/// Prevailing-torque hex nuts with a nylon collar; `m` is the height over
/// the collar.
pub const DIN_985: NutTable = NutTable {
    name: "DIN 985 / ISO 10511",
    rows: &[
        nut("M3", 3.0, 0.5, 5.5, 4.0),
        nut("M4", 4.0, 0.7, 7.0, 5.0),
        nut("M5", 5.0, 0.8, 8.0, 5.0),
        nut("M6", 6.0, 1.0, 10.0, 6.0),
        nut("M8", 8.0, 1.25, 13.0, 8.0),
        nut("M10", 10.0, 1.5, 16.0, 10.0),
        nut("M12", 12.0, 1.75, 18.0, 12.0),
        nut("M16", 16.0, 2.0, 24.0, 16.0),
        nut("M20", 20.0, 2.5, 30.0, 20.0),
    ],
};

/// Thin square nuts.
pub const DIN_562: NutTable = NutTable {
    name: "DIN 562 (thin)",
    rows: &[
        nut("M2", 2.0, 0.4, 4.0, 1.2),
        nut("M2.5", 2.5, 0.45, 5.0, 1.6),
        nut("M3", 3.0, 0.5, 5.5, 1.8),
        nut("M4", 4.0, 0.7, 7.0, 2.2),
        nut("M5", 5.0, 0.8, 8.0, 2.7),
        nut("M6", 6.0, 1.0, 10.0, 3.2),
        nut("M8", 8.0, 1.25, 13.0, 4.0),
        nut("M10", 10.0, 1.5, 17.0, 5.0),
    ],
};

/// Square nuts.
pub const DIN_557: NutTable = NutTable {
    name: "DIN 557",
    rows: &[
        nut("M5", 5.0, 0.8, 8.0, 4.0),
        nut("M6", 6.0, 1.0, 10.0, 5.0),
        nut("M8", 8.0, 1.25, 13.0, 6.5),
        nut("M10", 10.0, 1.5, 17.0, 8.0),
        nut("M12", 12.0, 1.75, 19.0, 10.0),
    ],
};

/// A T-slot nut as the extrusion maker draws it (Misumi 5, 6 and 8
/// series). Its section across the slot is a trapezoid: `top` wide under
/// the lips, straight down `straight`, then in to `bottom` wide on the
/// cavity's floor at `thick` deep. `length` runs along the slot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TNutRow {
    pub series: u32,
    pub kind: TNutKind,
    pub length: f64,
    pub top: f64,
    pub bottom: f64,
    pub thick: f64,
    pub straight: f64,
    /// Where the thread sits from one end along the slot; 0 for the middle.
    pub thread_at: f64,
    /// The thread sizes made for it.
    pub sizes: &'static [&'static str],
}

/// How a T-slot nut goes into its slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TNutKind {
    /// Slid in from the extrusion's end before assembly.
    Sliding,
    /// Dropped into the slot edgewise and turned flat.
    DropIn,
    /// A drop-in with a sprung ball under it, so it stays where it is put.
    SpringBall,
    /// Narrow enough to drop straight in, its ends cut at 15°; the screw's
    /// torque turns it to lock under the lips.
    Twist,
    /// A drop-in with a second, set-screw hole to lock it in place.
    RollIn,
}

#[allow(clippy::too_many_arguments)]
const fn tnut(
    series: u32,
    kind: TNutKind,
    length: f64,
    top: f64,
    bottom: f64,
    thick: f64,
    straight: f64,
    thread_at: f64,
    sizes: &'static [&'static str],
) -> TNutRow {
    TNutRow {
        series,
        kind,
        length,
        top,
        bottom,
        thick,
        straight,
        thread_at,
        sizes,
    }
}

const M3_5: &[&str] = &["M3", "M4", "M5"];
const M3_6: &[&str] = &["M3", "M4", "M5", "M6"];
const M4_8: &[&str] = &["M4", "M5", "M6", "M8"];

/// Misumi HNTT, HNTA, HNTP, HNTF and HNTR, by series. The 40 series'
/// sliding and twist nuts are catalogued 9 thick against a cavity drawn
/// 7 high; they are drawn to fit the cavity.
pub const T_NUTS: [TNutRow; 15] = [
    tnut(20, TNutKind::Sliding, 10.0, 10.0, 5.8, 4.1, 3.3, 0.0, M3_5),
    tnut(20, TNutKind::DropIn, 15.0, 8.0, 5.8, 3.2, 2.5, 0.0, M3_5),
    tnut(
        20,
        TNutKind::SpringBall,
        17.0,
        8.3,
        5.8,
        3.2,
        2.5,
        6.0,
        M3_5,
    ),
    tnut(
        20,
        TNutKind::Twist,
        10.0,
        6.0,
        5.8,
        4.1,
        3.3,
        0.0,
        &["M3", "M4"],
    ),
    tnut(20, TNutKind::RollIn, 17.0, 8.0, 5.8, 3.2, 2.4, 6.5, M3_5),
    tnut(30, TNutKind::Sliding, 14.0, 15.0, 7.8, 6.3, 5.5, 0.0, M3_6),
    tnut(30, TNutKind::DropIn, 17.0, 11.5, 7.8, 6.3, 5.5, 0.0, M3_6),
    tnut(
        30,
        TNutKind::SpringBall,
        17.0,
        11.5,
        7.8,
        6.3,
        5.5,
        6.0,
        M3_6,
    ),
    tnut(30, TNutKind::Twist, 15.0, 8.0, 7.8, 6.3, 5.5, 0.0, M3_6),
    tnut(30, TNutKind::RollIn, 17.0, 11.5, 7.8, 6.3, 5.5, 6.5, M3_6),
    tnut(40, TNutKind::Sliding, 17.0, 17.0, 9.8, 7.0, 5.0, 0.0, M4_8),
    tnut(40, TNutKind::DropIn, 20.0, 14.5, 9.8, 6.5, 4.5, 0.0, M4_8),
    tnut(
        40,
        TNutKind::SpringBall,
        20.0,
        14.5,
        9.8,
        6.5,
        4.5,
        8.0,
        M4_8,
    ),
    tnut(40, TNutKind::Twist, 17.0, 10.0, 9.8, 7.0, 5.0, 0.0, M4_8),
    tnut(40, TNutKind::RollIn, 20.0, 14.5, 9.8, 6.5, 4.5, 6.5, M4_8),
];

pub fn tnut_row(series: u32, kind: TNutKind) -> Option<&'static TNutRow> {
    T_NUTS.iter().find(|r| r.series == series && r.kind == kind)
}

/// A washer size: hole, outside and thickness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WasherRow {
    pub size: &'static str,
    pub d1: f64,
    pub d2: f64,
    pub h: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WasherTable {
    pub name: &'static str,
    pub rows: &'static [WasherRow],
}

const fn washer(size: &'static str, d1: f64, d2: f64, h: f64) -> WasherRow {
    WasherRow { size, d1, d2, h }
}

/// Plain washers, normal series.
pub const ISO_7089: WasherTable = WasherTable {
    name: "ISO 7089 / DIN 125",
    rows: &[
        washer("M2", 2.2, 5.0, 0.3),
        washer("M2.5", 2.7, 6.0, 0.5),
        washer("M3", 3.2, 7.0, 0.5),
        washer("M4", 4.3, 9.0, 0.8),
        washer("M5", 5.3, 10.0, 1.0),
        washer("M6", 6.4, 12.0, 1.6),
        washer("M8", 8.4, 16.0, 1.6),
        washer("M10", 10.5, 20.0, 2.0),
        washer("M12", 13.0, 24.0, 2.5),
        washer("M16", 17.0, 30.0, 3.0),
        washer("M20", 21.0, 37.0, 3.0),
    ],
};

/// Plain washers, large series.
pub const ISO_7093: WasherTable = WasherTable {
    name: "ISO 7093 / DIN 9021",
    rows: &[
        washer("M3", 3.2, 9.0, 0.8),
        washer("M4", 4.3, 12.0, 1.0),
        washer("M5", 5.3, 15.0, 1.2),
        washer("M6", 6.4, 18.0, 1.6),
        washer("M8", 8.4, 24.0, 2.0),
        washer("M10", 10.5, 30.0, 2.5),
        washer("M12", 13.0, 37.0, 3.0),
        washer("M16", 17.0, 50.0, 3.0),
        washer("M20", 21.0, 60.0, 4.0),
    ],
};

/// Split spring lock washers.
pub const DIN_127: WasherTable = WasherTable {
    name: "DIN 127 B",
    rows: &[
        washer("M3", 3.1, 6.2, 0.8),
        washer("M4", 4.1, 7.6, 0.9),
        washer("M5", 5.1, 9.2, 1.2),
        washer("M6", 6.1, 11.8, 1.6),
        washer("M8", 8.1, 14.8, 2.0),
        washer("M10", 10.2, 18.1, 2.2),
        washer("M12", 12.2, 21.1, 2.5),
        washer("M16", 16.2, 27.4, 3.5),
        washer("M20", 20.2, 33.6, 4.0),
    ],
};

/// A T-slot extrusion series, as Misumi draws its 5, 6 and 8 series
/// (20, 30 and 40 mm cells with 6, 8 and 10 mm slots). Every open face of
/// a cell has the slot: `opening` wide at the surface through a lip
/// `lip` thick, then a cavity `cavity` wide down to `shoulder` from the
/// surface, where its walls run in at 45° to the floor at `depth`. A
/// hole runs down each cell; the 30 series has a hole in each corner
/// block and the 40 series a hollow one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtrusionSeries {
    pub cell: u32,
    pub opening: f64,
    pub lip: f64,
    pub cavity: f64,
    pub shoulder: f64,
    pub depth: f64,
    /// The floor's width: 0 for what 45° walls from the shoulder leave.
    pub floor: f64,
    pub hole: f64,
    pub corner: f64,
    /// A hollow right triangle in each outer corner block, its right angle
    /// a lip's thickness in from both faces: the legs' length; 0 for none.
    pub corner_tri: f64,
    /// A hole in each outer corner block: its diameter and its centre's
    /// distance in from both outer faces; 0 for none.
    pub corner_hole: f64,
    pub corner_hole_in: f64,
    /// A hollow in each outer corner block: its side and the wall around
    /// it; 0 for none.
    pub corner_void: f64,
    pub corner_wall: f64,
    /// The wall between an interior void and a cavity's floor.
    pub web: f64,
    /// Section area and mass per metre, as catalogued, for the checks.
    pub area_mm2: f64,
}

pub const EXTRUSIONS: [ExtrusionSeries; 3] = [
    ExtrusionSeries {
        cell: 20,
        opening: 6.0,
        lip: 2.0,
        cavity: 12.0,
        shoulder: 3.0,
        depth: 6.0,
        floor: 0.0,
        hole: 4.2,
        corner: 1.0,
        corner_tri: 0.0,
        corner_hole: 0.0,
        corner_hole_in: 0.0,
        corner_void: 0.0,
        corner_wall: 0.0,
        web: 1.5,
        area_mm2: 183.0,
    },
    ExtrusionSeries {
        cell: 30,
        opening: 8.0,
        lip: 2.0,
        cavity: 16.5,
        shoulder: 4.75,
        depth: 9.0,
        floor: 0.0,
        hole: 6.8,
        corner: 2.0,
        corner_tri: 0.0,
        corner_hole: 4.2,
        corner_hole_in: 3.4,
        corner_void: 0.0,
        corner_wall: 0.0,
        web: 2.0,
        area_mm2: 347.0,
    },
    ExtrusionSeries {
        cell: 40,
        opening: 10.0,
        lip: 5.5,
        cavity: 20.0,
        shoulder: 7.5,
        depth: 12.5,
        floor: 0.0,
        hole: 10.5,
        corner: 3.0,
        corner_tri: 0.0,
        corner_hole: 0.0,
        corner_hole_in: 0.0,
        corner_void: 6.0,
        corner_wall: 2.0,
        web: 2.0,
        area_mm2: 760.0,
    },
];

/// OpenBuilds' V-slot 20 series: the lips cut back at 45° to a 5.68
/// throat 1.8 deep, an 11 cavity with 1.64 shoulders closing at 45° to a
/// 5.68 floor 6.1 deep, and a hollow triangle behind each corner.
pub const VSLOT_20: ExtrusionSeries = ExtrusionSeries {
    cell: 20,
    opening: 5.68,
    lip: 1.8,
    cavity: 11.0,
    shoulder: 3.44,
    depth: 6.1,
    floor: 5.68,
    hole: 4.2,
    corner: 1.5,
    corner_tri: 1.63,
    corner_hole: 0.0,
    corner_hole_in: 0.0,
    corner_void: 0.0,
    corner_wall: 0.0,
    web: 1.5,
    area_mm2: 164.0,
};

pub fn extrusion(cell: u32) -> Option<&'static ExtrusionSeries> {
    EXTRUSIONS.iter().find(|s| s.cell == cell)
}

/// The series a profile is cut from: the V-slot's for a 20 series with
/// V lips, else the cell's.
pub fn extrusion_series(cell: u32, v_slot: bool) -> Option<&'static ExtrusionSeries> {
    if v_slot && cell == 20 {
        Some(&VSLOT_20)
    } else {
        extrusion(cell)
    }
}

/// A series' floor width: as given, or what 45° walls from the shoulder
/// leave.
pub fn series_floor(s: &ExtrusionSeries) -> f64 {
    if s.floor > 0.0 {
        s.floor
    } else {
        s.cavity - 2.0 * (s.depth - s.shoulder)
    }
}

/// A heat-set threaded insert for printed parts, as the common brass
/// ones are made (CNC Kitchen, Ruthex): a pilot end that finds the hole,
/// two knurled bands, the upper the wider, and the hole to drive it into,
/// which the pilot fits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InsertRow {
    pub size: &'static str,
    pub d: f64,
    pub pitch: f64,
    /// Across the upper knurl.
    pub outer: f64,
    /// Across the pilot end and the lower knurl's root.
    pub pilot: f64,
    pub length: f64,
    pub short: f64,
    pub hole: f64,
}

const fn insert(
    size: &'static str,
    d: f64,
    pitch: f64,
    outer: f64,
    pilot: f64,
    length: f64,
    short: f64,
) -> InsertRow {
    InsertRow {
        size,
        d,
        pitch,
        outer,
        pilot,
        length,
        short,
        hole: pilot,
    }
}

pub const INSERTS: [InsertRow; 6] = [
    insert("M2", 2.0, 0.4, 3.5, 3.2, 4.0, 3.0),
    insert("M2.5", 2.5, 0.45, 4.0, 3.6, 5.7, 4.0),
    insert("M3", 3.0, 0.5, 4.6, 4.0, 5.7, 4.0),
    insert("M4", 4.0, 0.7, 6.0, 5.6, 8.1, 4.7),
    insert("M5", 5.0, 0.8, 7.1, 6.4, 9.5, 5.8),
    insert("M6", 6.0, 1.0, 8.4, 8.0, 12.7, 8.0),
];

/// A rolling bearing by its designation: bore, outside and width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BearingRow {
    pub name: &'static str,
    pub bore: f64,
    pub outer: f64,
    pub width: f64,
}

const fn bearing(name: &'static str, bore: f64, outer: f64, width: f64) -> BearingRow {
    BearingRow {
        name,
        bore,
        outer,
        width,
    }
}

/// Deep groove ball bearings, the sizes printed designs reach for; the
/// DIN 625 rows agree with BOLTS' table of the standard.
pub const BALL_BEARINGS: [BearingRow; 35] = [
    bearing("MR85", 5.0, 8.0, 2.5),
    bearing("MR105", 5.0, 10.0, 4.0),
    bearing("MR115", 5.0, 11.0, 4.0),
    bearing("683", 3.0, 7.0, 3.0),
    bearing("693", 3.0, 8.0, 4.0),
    bearing("623", 3.0, 10.0, 4.0),
    bearing("604", 4.0, 12.0, 4.0),
    bearing("624", 4.0, 13.0, 5.0),
    bearing("625", 5.0, 16.0, 5.0),
    bearing("626", 6.0, 19.0, 6.0),
    bearing("688", 8.0, 16.0, 5.0),
    bearing("698", 8.0, 19.0, 6.0),
    bearing("608", 8.0, 22.0, 7.0),
    bearing("6700", 10.0, 15.0, 4.0),
    bearing("6800", 10.0, 19.0, 5.0),
    bearing("6900", 10.0, 22.0, 6.0),
    bearing("6000", 10.0, 26.0, 8.0),
    bearing("6200", 10.0, 30.0, 9.0),
    bearing("6701", 12.0, 18.0, 4.0),
    bearing("6801", 12.0, 21.0, 5.0),
    bearing("6001", 12.0, 28.0, 8.0),
    bearing("6201", 12.0, 32.0, 10.0),
    bearing("6802", 15.0, 24.0, 5.0),
    bearing("6002", 15.0, 32.0, 9.0),
    bearing("607", 7.0, 19.0, 6.0),
    bearing("609", 9.0, 24.0, 7.0),
    bearing("627", 7.0, 22.0, 7.0),
    bearing("629", 9.0, 26.0, 8.0),
    bearing("6003", 17.0, 35.0, 10.0),
    bearing("6004", 20.0, 42.0, 12.0),
    bearing("6005", 25.0, 47.0, 12.0),
    bearing("6202", 15.0, 35.0, 11.0),
    bearing("6203", 17.0, 40.0, 12.0),
    bearing("6204", 20.0, 47.0, 14.0),
    bearing("6205", 25.0, 52.0, 15.0),
];

/// Linear ball bushings for round shafts.
pub const LINEAR_BEARINGS: [BearingRow; 8] = [
    bearing("LM6UU", 6.0, 12.0, 19.0),
    bearing("LM8UU", 8.0, 15.0, 24.0),
    bearing("LM8LUU", 8.0, 15.0, 45.0),
    bearing("LM10UU", 10.0, 19.0, 29.0),
    bearing("LM12UU", 12.0, 21.0, 30.0),
    bearing("LM12LUU", 12.0, 21.0, 57.0),
    bearing("LM16UU", 16.0, 28.0, 37.0),
    bearing("LM20UU", 20.0, 32.0, 42.0),
];

/// Common disc magnets, diameter × height.
pub const DISC_MAGNETS: [(f64, f64); 14] = [
    (3.0, 1.0),
    (3.0, 2.0),
    (4.0, 2.0),
    (5.0, 2.0),
    (6.0, 2.0),
    (6.0, 3.0),
    (8.0, 2.0),
    (8.0, 3.0),
    (10.0, 2.0),
    (10.0, 3.0),
    (12.0, 3.0),
    (15.0, 3.0),
    (20.0, 3.0),
    (20.0, 5.0),
];

/// Common block magnets, length × width × height.
pub const BLOCK_MAGNETS: [(f64, f64, f64); 6] = [
    (10.0, 5.0, 2.0),
    (10.0, 5.0, 3.0),
    (20.0, 5.0, 2.0),
    (20.0, 10.0, 2.0),
    (25.0, 10.0, 3.0),
    (40.0, 10.0, 5.0),
];

/// Shaft diameters as drawn and ground rod is sold.
pub const SHAFT_DIAMETERS: [f64; 9] = [3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 12.0, 16.0, 20.0];

/// Dowel pin diameters (ISO 8734).
pub const DOWEL_DIAMETERS: [f64; 8] = [2.0, 2.5, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0];

/// Trapezoidal lead screws by designation: diameter, pitch and lead.
pub const LEAD_SCREWS: [(&str, f64, f64, f64); 4] = [
    ("T8 (lead 8)", 8.0, 2.0, 8.0),
    ("T8 (lead 2)", 8.0, 2.0, 2.0),
    ("T8 (lead 4)", 8.0, 2.0, 4.0),
    ("T12 (lead 3)", 12.0, 3.0, 3.0),
];

/// The minor diameter of an internal metric thread of `d` and `pitch`.
pub fn internal_minor(d: f64, pitch: f64) -> f64 {
    d - 2.0 * crate::geom::THREAD_DEPTH * pitch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_sizes_read_with_their_pitches() {
        assert_eq!(metric("M3"), Some((3.0, 0.5)));
        assert_eq!(metric("m8"), Some((8.0, 1.25)));
        assert_eq!(metric("M7"), None);
    }

    #[test]
    fn every_table_row_is_sized_sensibly() {
        for table in [
            ISO_4762, ASME_B18_3, ISO_7380, ISO_10642, ISO_4017, DIN_7984, ISO_4026,
        ] {
            for row in table.rows {
                assert!(
                    row.d > 0.0 && row.pitch > 0.0,
                    "{} {}",
                    table.name,
                    row.size
                );
                assert!(row.pitch < row.d, "{} {}", table.name, row.size);
                if row.k > 0.0 {
                    assert!(row.dk > row.d, "{} {}", table.name, row.size);
                }
                if row.s > 0.0 && table.name != ISO_4026.name {
                    assert!(row.s < row.dk, "{} {}", table.name, row.size);
                }
            }
        }
        for table in [ISO_4032, ISO_4035, DIN_985, DIN_562, DIN_557] {
            for row in table.rows {
                assert!(row.s > row.d && row.m > 0.0, "{} {}", table.name, row.size);
            }
        }
        for table in [ISO_7089, ISO_7093, DIN_127] {
            for row in table.rows {
                assert!(
                    row.d2 > row.d1 && row.h > 0.0,
                    "{} {}",
                    table.name,
                    row.size
                );
            }
        }
        for series in EXTRUSIONS.iter().chain([&VSLOT_20]) {
            assert!(series.cavity > series.opening);
            assert!(series.depth > series.shoulder && series.shoulder > series.lip);
            let floor = series_floor(series);
            assert!(
                floor > 0.0,
                "{}: the walls meet before the floor",
                series.cell
            );
            // Every nut of the series sits on the floor under the lips (the
            // V-slot's nuts ride its sloped walls, so they are not checked).
            for nut in T_NUTS
                .iter()
                .filter(|n| series.floor == 0.0 && n.series == series.cell)
            {
                assert!(nut.bottom <= floor + 0.01, "{} {:?}", series.cell, nut.kind);
                assert!(nut.top < series.cavity, "{} {:?}", series.cell, nut.kind);
                assert!(
                    nut.thick <= series.depth - series.lip + 0.11,
                    "{} {:?}",
                    series.cell,
                    nut.kind
                );
                assert!(nut.straight < nut.thick, "{} {:?}", series.cell, nut.kind);
            }
        }
        for row in INSERTS {
            assert!(row.outer > row.pilot && row.pilot > row.d && row.short <= row.length);
        }
        for row in BALL_BEARINGS.iter().chain(&LINEAR_BEARINGS) {
            assert!(row.outer > row.bore && row.width > 0.0, "{}", row.name);
        }
    }
}
