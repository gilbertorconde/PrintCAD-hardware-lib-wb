//! Every part's features, checked against each other: each boolean step
//! after the first is built on its own, and every pair is intersected.
//! Two features that share volume are listed, with how much, so a hole
//! cut through a ball, or a set screw through a thread, shows up before
//! a picture does. Some pairs are meant to meet: a thread's groove and
//! its bore, a socket and its head, a knurl band and its core. The
//! table is for reading, not a pass or fail.

#[path = "../cases.rs"]
mod cases;
#[path = "../../../../src/diagram.rs"]
pub mod diagram;
#[path = "../../../../src/geom.rs"]
pub mod geom;
#[path = "../../../../src/parts/mod.rs"]
pub mod parts;
#[path = "../../../../src/standards.rs"]
pub mod standards;

use kernel_api::{BooleanOp, SolidOp, TessellationSettings};

/// The op as a solid of its own.
fn standalone(op: &SolidOp) -> Option<SolidOp> {
    let mut own = op.clone();
    let shaped = match &mut own {
        SolidOp::Sweep { op, .. }
        | SolidOp::SweepFace { op, .. }
        | SolidOp::SweepFaceOf { op, .. }
        | SolidOp::Loft { op, .. }
        | SolidOp::LoftThrough { op, .. }
        | SolidOp::Pipe { op, .. }
        | SolidOp::PipeThrough { op, .. }
        | SolidOp::Primitive { op, .. } => {
            *op = BooleanOp::NewSolid;
            true
        }
        _ => false,
    };
    shaped.then_some(own)
}

fn with_op(op: &SolidOp, role: BooleanOp) -> SolidOp {
    let mut own = op.clone();
    if let SolidOp::Sweep { op, .. }
    | SolidOp::SweepFace { op, .. }
    | SolidOp::SweepFaceOf { op, .. }
    | SolidOp::Loft { op, .. }
    | SolidOp::LoftThrough { op, .. }
    | SolidOp::Pipe { op, .. }
    | SolidOp::PipeThrough { op, .. }
    | SolidOp::Primitive { op, .. } = &mut own
    {
        *op = role;
    }
    own
}

fn what(op: &SolidOp) -> String {
    let kind = match op {
        SolidOp::Sweep { .. } => "sweep",
        SolidOp::Primitive { .. } => "primitive",
        SolidOp::Loft { .. } | SolidOp::LoftThrough { .. } => "loft",
        SolidOp::Pipe { .. } | SolidOp::PipeThrough { .. } => "pipe",
        _ => "other",
    };
    format!(
        "{kind} {:?}",
        op.boolean_op().unwrap_or(BooleanOp::NewSolid)
    )
}

fn main() {
    let mut kernel = kernel_ogeom::OgeomKernel::new();
    let detail = TessellationSettings::default();
    let mut volume = |ops: &[SolidOp]| -> Option<f64> {
        let built = kernel.execute_solid_chain(ops, &detail).ok()?;
        kernel
            .physical_properties(&built.brep_blob)
            .ok()
            .and_then(|p| p.volume_mm3)
    };
    let mut pairs = 0;
    for case in cases::cases() {
        let ops = case.part.ops();
        let tools: Vec<(usize, SolidOp, f64)> = ops
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(i, op)| {
                let own = standalone(op)?;
                let v = volume(std::slice::from_ref(&own)).unwrap_or(0.0);
                Some((i, own, v))
            })
            .collect();
        let mut lines = Vec::new();
        for (a, (i, own_i, vi)) in tools.iter().enumerate() {
            for (j, own_j, vj) in &tools[a + 1..] {
                let shared =
                    volume(&[own_i.clone(), with_op(own_j, BooleanOp::Common)]).unwrap_or(0.0);
                if shared > 1e-3 {
                    lines.push(format!(
                        "    op {i} ({}, {vi:.2} mm³) ∩ op {j} ({}, {vj:.2} mm³) = {shared:.2} mm³",
                        what(&ops[*i]),
                        what(&ops[*j])
                    ));
                }
            }
        }
        if !lines.is_empty() {
            pairs += lines.len();
            println!("{}", case.name);
            for line in lines {
                println!("{line}");
            }
        }
    }
    println!();
    println!("{pairs} pairs of features share volume");
}
