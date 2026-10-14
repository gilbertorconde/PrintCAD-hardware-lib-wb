//! A part's declared threads, found on the built mesh as the host finds
//! them: the face of the triangle nearest each thread's point, which must
//! be a round face whose radius is the point's distance from its axis.

use kernel_api::{FaceSurface, TriMesh};

use crate::parts::Thread;

type V = [f64; 3];

fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn add_scaled(a: V, b: V, s: f64) -> V {
    [a[0] + b[0] * s, a[1] + b[1] * s, a[2] + b[2] * s]
}
fn f64s(p: [f32; 3]) -> V {
    p.map(f64::from)
}

/// The squared distance from `p` to the triangle `a b c`.
fn triangle_distance_sq(p: V, a: V, b: V, c: V) -> f64 {
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    let nearest = if d1 <= 0.0 && d2 <= 0.0 {
        a
    } else {
        let bp = sub(p, b);
        let (d3, d4) = (dot(ab, bp), dot(ac, bp));
        let cp = sub(p, c);
        let (d5, d6) = (dot(ab, cp), dot(ac, cp));
        let vc = d1 * d4 - d3 * d2;
        let vb = d5 * d2 - d1 * d6;
        let va = d3 * d6 - d5 * d4;
        if d3 >= 0.0 && d4 <= d3 {
            b
        } else if d6 >= 0.0 && d5 <= d6 {
            c
        } else if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
            add_scaled(a, ab, d1 / (d1 - d3))
        } else if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
            add_scaled(a, ac, d2 / (d2 - d6))
        } else if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
            add_scaled(b, sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6)))
        } else {
            let denom = 1.0 / (va + vb + vc);
            add_scaled(add_scaled(a, ab, vb * denom), ac, vc * denom)
        }
    };
    let d = sub(p, nearest);
    dot(d, d)
}

/// Where `thread` lands on `mesh`: `inside` or `outside` and the face's
/// radius, or why it lands on no round face.
pub fn land(mesh: &TriMesh, thread: &Thread) -> Result<String, String> {
    let corner = |t: usize, k: usize| f64s(mesh.positions[mesh.indices[3 * t + k] as usize]);
    let face = (0..mesh.faces.len())
        .map(|t| {
            let d = triangle_distance_sq(thread.point, corner(t, 0), corner(t, 1), corner(t, 2));
            (d, mesh.faces[t])
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, f)| f)
        .ok_or("no triangles")?;
    let Some(FaceSurface::Cylinder {
        origin,
        axis,
        radius,
    }) = mesh.face_surfaces.get(face as usize).copied()
    else {
        return Err(format!(
            "{} lands on face {face}, {:?}",
            thread.designation,
            mesh.face_surfaces.get(face as usize)
        ));
    };
    let (origin, axis, radius) = (f64s(origin), f64s(axis), f64::from(radius));
    let axis = {
        let l = dot(axis, axis).sqrt();
        axis.map(|v| v / l)
    };
    let off = sub(thread.point, origin);
    let radial = add_scaled(off, axis, -dot(off, axis));
    let distance = dot(radial, radial).sqrt();
    if (distance - radius).abs() > 1e-3 * radius.max(1.0) {
        return Err(format!(
            "{} at {:.3} from the axis of a face of radius {radius:.3}",
            thread.designation, distance
        ));
    }
    // A bore's normals face its axis, a shank's away.
    let mut facing = 0.0;
    for (t, f) in mesh.faces.iter().enumerate() {
        if *f != face {
            continue;
        }
        for k in 0..3 {
            let i = mesh.indices[3 * t + k] as usize;
            let p = sub(f64s(mesh.positions[i]), origin);
            let r = add_scaled(p, axis, -dot(p, axis));
            if let Some(n) = mesh.normals.get(i) {
                facing += dot(f64s(*n), r);
            }
        }
    }
    let side = if facing < 0.0 { "inside" } else { "outside" };
    Ok(format!(
        "{} {side} Ø{:.2}",
        thread.designation,
        2.0 * radius
    ))
}
