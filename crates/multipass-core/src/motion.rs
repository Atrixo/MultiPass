use crate::ncfile::{COMMENT, TOOLCHANGE};
use crate::pycompat::{parse_float, parse_uint, regex, strip};
use regex::Regex;
use serde::Serialize;
use std::f64::consts::{PI, TAU};
use std::sync::LazyLock;

static WORD: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)([A-Z])\s*([-+]?(?:\d+\.?\d*|\.\d+))"));

const MM_PER_INCH: f64 = 25.4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    Rapid,
    Cut,
}

#[derive(Debug, Clone, Copy)]
pub struct Move {
    pub kind: MoveKind,
    pub tool: Option<u64>,
    pub from: [f64; 3],
    pub to: [f64; 3],
    pub arc: Option<Arc>,
    pub length: f64,
    pub feed: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub struct Arc {
    pub center: [f64; 2],
    pub radius: f64,
    pub start_angle: f64,
    pub sweep: f64,
}

impl Arc {
    pub fn point(&self, t: f64) -> [f64; 2] {
        let a = self.start_angle + self.sweep * t;
        [self.center[0] + self.radius * a.cos(), self.center[1] + self.radius * a.sin()]
    }
}

#[derive(Default)]
struct Line {
    g: Vec<f64>,
    m: Vec<f64>,
    xyz: [Option<f64>; 3],
    ijk: [Option<f64>; 3],
    r: Option<f64>,
    f: Option<f64>,
}

fn parse(code: &str) -> Line {
    let mut l = Line::default();
    for c in WORD.captures_iter(code) {
        let Some(v) = parse_float(&c[2]) else { continue };
        match c[1].chars().next().unwrap_or(' ').to_ascii_uppercase() {
            'G' => l.g.push(v),
            'M' => l.m.push(v),
            'X' => l.xyz[0] = Some(v),
            'Y' => l.xyz[1] = Some(v),
            'Z' => l.xyz[2] = Some(v),
            'I' => l.ijk[0] = Some(v),
            'J' => l.ijk[1] = Some(v),
            'K' => l.ijk[2] = Some(v),
            'R' => l.r = Some(v),
            'F' => l.f = Some(v),
            _ => {}
        }
    }
    l
}

fn norm_angle(a: f64) -> f64 {
    let r = a % TAU;
    if r < 0.0 {
        r + TAU
    } else {
        r
    }
}

fn arc_xy(from: [f64; 3], to: [f64; 3], ijk: [Option<f64>; 3], r: Option<f64>, cw: bool) -> Option<Arc> {
    let (sx, sy, ex, ey) = (from[0], from[1], to[0], to[1]);
    let center = if ijk[0].is_some() || ijk[1].is_some() {
        [sx + ijk[0].unwrap_or(0.0), sy + ijk[1].unwrap_or(0.0)]
    } else {
        let r = r?;
        let (dx, dy) = (ex - sx, ey - sy);
        let d = (dx * dx + dy * dy).sqrt();
        if d < 1e-12 || d > 2.0 * r.abs() + 1e-9 {
            return None;
        }
        let h = (r * r - d * d / 4.0).max(0.0).sqrt();
        let (mx, my) = (sx + dx / 2.0, sy + dy / 2.0);
        let (ux, uy) = (-dy / d, dx / d);
        let left = (cw && r < 0.0) || (!cw && r > 0.0);
        if left {
            [mx + ux * h, my + uy * h]
        } else {
            [mx - ux * h, my - uy * h]
        }
    };
    let radius = ((sx - center[0]).powi(2) + (sy - center[1]).powi(2)).sqrt();
    if radius < 1e-12 {
        return None;
    }
    let a0 = (sy - center[1]).atan2(sx - center[0]);
    let a1 = (ey - center[1]).atan2(ex - center[0]);
    let same = (sx - ex).abs() < 1e-9 && (sy - ey).abs() < 1e-9;
    let mut sweep = if cw { norm_angle(a0 - a1) } else { norm_angle(a1 - a0) };
    if same || sweep < 1e-12 {
        sweep = TAU;
    }
    Some(Arc { center, radius, start_angle: a0, sweep: if cw { -sweep } else { sweep } })
}

pub struct Simulation {
    pub moves: Vec<Move>,
    pub inch: bool,
}

pub fn simulate<S: AsRef<str>>(lines: &[S], inch_default: bool) -> Simulation {
    let mut moves = Vec::new();
    let mut mode: Option<i32> = None;
    let mut pos: [Option<f64>; 3] = [None; 3];
    let mut feed: Option<f64> = None;
    let mut absolute = true;
    let mut inch = inch_default;
    let mut plane = 17;
    let mut tool: Option<u64> = None;
    for line in lines {
        let line = line.as_ref();
        if let Some(c) = TOOLCHANGE.captures(line) {
            tool = parse_uint(&c[1]);
        }
        let without = COMMENT.replace_all(line, "");
        let code = strip(&without);
        if code.is_empty() {
            continue;
        }
        let l = parse(code);
        for &g in &l.g {
            if [0.0, 1.0, 2.0, 3.0].contains(&g) {
                mode = Some(g as i32);
            } else if [17.0, 18.0, 19.0].contains(&g) {
                plane = g as i32;
            } else if g == 20.0 || g == 21.0 {
                inch = g == 20.0;
            } else if g == 90.0 || g == 91.0 {
                absolute = g == 90.0;
            }
        }
        if let Some(f) = l.f {
            feed = Some(f);
        }
        let reset = l.g.iter().any(|g| *g == 28.0 || *g == 30.0 || *g == 53.0);
        let non_motion = l.g.iter().any(|g| [4.0, 10.0, 92.0].contains(g));
        if reset || non_motion {
            if reset {
                pos = [None; 3];
            }
            continue;
        }
        if l.xyz.iter().all(Option::is_none) {
            continue;
        }
        let mut next = pos;
        for a in 0..3 {
            if let Some(v) = l.xyz[a] {
                next[a] = if absolute { Some(v) } else { pos[a].map(|p| p + v) };
            }
        }
        let known = |p: [Option<f64>; 3]| -> Option<[f64; 3]> { Some([p[0]?, p[1]?, p[2]?]) };
        if let (Some(m), Some(from), Some(to)) = (mode, known(pos), known(next)) {
            let scale = if inch { MM_PER_INCH } else { 1.0 };
            let (from_mm, to_mm) = (from.map(|v| v * scale), to.map(|v| v * scale));
            let straight =
                ((to_mm[0] - from_mm[0]).powi(2) + (to_mm[1] - from_mm[1]).powi(2) + (to_mm[2] - from_mm[2]).powi(2))
                    .sqrt();
            let mut arc = None;
            let mut length = straight;
            if (m == 2 || m == 3) && plane == 17 {
                let ijk = l.ijk.map(|v| v.map(|x| x * scale));
                if let Some(a) = arc_xy(from_mm, to_mm, ijk, l.r.map(|r| r * scale), m == 2) {
                    let dz = to_mm[2] - from_mm[2];
                    length = ((a.radius * a.sweep).powi(2) + dz * dz).sqrt();
                    arc = Some(a);
                }
            }
            let kind = if m == 0 { MoveKind::Rapid } else { MoveKind::Cut };
            let feed_mm = feed.map(|f| f * scale);
            if length > 0.0 {
                moves.push(Move { kind, tool, from: from_mm, to: to_mm, arc, length, feed: feed_mm });
            }
        }
        pos = next;
    }
    Simulation { moves, inch }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Estimate {
    pub cut_mm: f64,
    pub cut_min: f64,
    pub rapid_mm: f64,
    pub moves: usize,
    pub unknown_feed_moves: usize,
}

impl Estimate {
    pub fn total_min(&self, rapid_mm_per_min: f64) -> f64 {
        self.cut_min + if rapid_mm_per_min > 0.0 { self.rapid_mm / rapid_mm_per_min } else { 0.0 }
    }
}

pub fn estimate<S: AsRef<str>>(lines: &[S], inch: bool) -> Estimate {
    let sim = simulate(lines, inch);
    let mut e = Estimate::default();
    for m in &sim.moves {
        e.moves += 1;
        match m.kind {
            MoveKind::Rapid => e.rapid_mm += m.length,
            MoveKind::Cut => {
                e.cut_mm += m.length;
                match m.feed.filter(|f| *f > 0.0) {
                    Some(f) => e.cut_min += m.length / f,
                    None => e.unknown_feed_moves += 1,
                }
            }
        }
    }
    e
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Polyline {
    pub rapid: bool,
    pub tool: Option<u64>,
    pub points: Vec<f32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub polylines: Vec<Polyline>,
    pub bounds: Option<[f64; 4]>,
    pub points: usize,
}

fn arc_points(m: &Move) -> Vec<[f64; 2]> {
    match m.arc {
        Some(a) => {
            let n = ((a.sweep.abs() / (PI / 24.0)).ceil() as usize).clamp(2, 512);
            (1..=n).map(|i| a.point(i as f64 / n as f64)).collect()
        }
        None => vec![[m.to[0], m.to[1]]],
    }
}

pub fn preview<S: AsRef<str>>(lines: &[S], inch: bool, max_points: usize) -> Preview {
    let sim = simulate(lines, inch);
    let mut bounds: Option<[f64; 4]> = None;
    let mut grow = |p: [f64; 2]| {
        let b = bounds.get_or_insert([p[0], p[1], p[0], p[1]]);
        b[0] = b[0].min(p[0]);
        b[1] = b[1].min(p[1]);
        b[2] = b[2].max(p[0]);
        b[3] = b[3].max(p[1]);
    };
    for m in &sim.moves {
        grow([m.from[0], m.from[1]]);
        for p in arc_points(m) {
            grow(p);
        }
    }
    let Some(b) = bounds else {
        return Preview { polylines: Vec::new(), bounds: None, points: 0 };
    };
    let diag = ((b[2] - b[0]).powi(2) + (b[3] - b[1]).powi(2)).sqrt().max(1e-6);
    let mut eps = diag / 2000.0;
    loop {
        let out = build_polylines(&sim.moves, eps);
        let points: usize = out.iter().map(|p| p.points.len() / 2).sum();
        if points <= max_points || eps > diag {
            return Preview { polylines: out, bounds: Some(b), points };
        }
        eps *= 2.0;
    }
}

fn near(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-9 && (a[1] - b[1]).abs() < 1e-9
}

fn build_polylines(moves: &[Move], eps: f64) -> Vec<Polyline> {
    let eps2 = eps * eps;
    let mut out: Vec<Polyline> = Vec::new();
    let mut cursor: Option<[f64; 2]> = None;
    let mut emitted = [0.0, 0.0];
    let mut pending = false;
    for m in moves {
        let rapid = m.kind == MoveKind::Rapid;
        let start = [m.from[0], m.from[1]];
        let continues =
            out.last().is_some_and(|p| p.rapid == rapid && p.tool == m.tool) && cursor.is_some_and(|c| near(c, start));
        if !continues {
            if let (true, Some(c), Some(p)) = (pending, cursor, out.last_mut()) {
                p.points.extend([c[0] as f32, c[1] as f32]);
            }
            out.push(Polyline { rapid, tool: m.tool, points: vec![start[0] as f32, start[1] as f32] });
            emitted = start;
            pending = false;
        }
        for p in arc_points(m) {
            if (p[0] - emitted[0]).powi(2) + (p[1] - emitted[1]).powi(2) >= eps2 {
                if let Some(cur) = out.last_mut() {
                    cur.points.extend([p[0] as f32, p[1] as f32]);
                }
                emitted = p;
                pending = false;
            } else {
                pending = true;
            }
            cursor = Some(p);
        }
    }
    if let (true, Some(c), Some(p)) = (pending, cursor, out.last_mut()) {
        p.points.extend([c[0] as f32, c[1] as f32]);
    }
    out.retain(|p| p.points.len() >= 4);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn est(src: &str) -> Estimate {
        let lines: Vec<&str> = src.lines().collect();
        estimate(&lines, false)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn straight_moves() {
        let e = est("G90\nG0 X0 Y0 Z5\nX10\nG1 Z0 F100\nX40 Y40 F500");
        assert!(close(e.rapid_mm, 10.0));
        assert!(close(e.cut_mm, 5.0 + 50.0));
        assert!(close(e.cut_min, 5.0 / 100.0 + 50.0 / 500.0));
        assert_eq!(e.total_min(1000.0), e.cut_min + 0.01);
    }

    #[test]
    fn arcs_with_ij_and_r() {
        let q = PI * 10.0 / 2.0;
        let e = est("G0 X10 Y0 Z0\nG3 X0 Y10 I-10 J0 F100");
        assert!(close(e.cut_mm, q), "{}", e.cut_mm);
        let e = est("G0 X10 Y0 Z0\nG2 X0 Y10 I-10 J0 F100");
        assert!(close(e.cut_mm, 3.0 * q), "{}", e.cut_mm);
        let e = est("G0 X10 Y0 Z0\nG3 X0 Y10 R10 F100");
        assert!(close(e.cut_mm, q), "{}", e.cut_mm);
        let e = est("G0 X10 Y0 Z0\nG3 X0 Y10 R-10 F100");
        assert!(close(e.cut_mm, 3.0 * q), "{}", e.cut_mm);
        let e = est("G0 X10 Y0 Z0\nG2 X10 Y0 I-10 J0 F100");
        assert!(close(e.cut_mm, 2.0 * PI * 10.0), "{}", e.cut_mm);
        let e = est("G0 X10 Y0 Z0\nG3 X0 Y10 Z-3 I-10 J0 F100");
        assert!(close(e.cut_mm, (q * q + 9.0).sqrt()));
    }

    #[test]
    fn incremental_inch_and_unknown() {
        let e = est("G91\nG0 X0 Y0 Z0\nG90 G0 X0 Y0 Z0\nG91 G1 X3 Y4 F60");
        assert!(close(e.cut_mm, 5.0));
        let lines = ["G20", "G0 X0 Y0 Z0", "G1 X1 F10"];
        let e = estimate(&lines, false);
        assert!(close(e.cut_mm, 25.4));
        assert!(close(e.cut_min, 0.1));
        let e = est("G0 X0 Y0 Z0\nG1 X5\nG28\nG1 X9\nY3 Z1 F100\nX10");
        assert_eq!(e.unknown_feed_moves, 1);
        assert!(close(e.cut_mm, 6.0));
    }

    #[test]
    fn preview_is_thinned_and_split_by_tool() {
        let mut src = String::from("T1M6\nG0 X0 Y0 Z5\nG1 Z-1 F100\n");
        for i in 0..20000 {
            src.push_str(&format!("X{:.4} Y{:.4}\n", (i as f64 * 0.01).sin() * 50.0, i as f64 * 0.001));
        }
        src.push_str("G0 Z5\nT2M6\nG0 X100 Y100\nG1 Z-1\nG2 X100 Y100 I10 J0\n");
        let lines: Vec<&str> = src.lines().collect();
        let p = preview(&lines, false, 5000);
        assert!(p.points <= 5000, "{}", p.points);
        assert!(p.points > 100);
        assert!(p.polylines.iter().any(|l| l.tool == Some(2) && !l.rapid));
        assert!(p.polylines.iter().any(|l| l.rapid));
        let b = p.bounds.unwrap();
        assert!(b[2] >= 119.9 && b[0] <= -49.0);
    }
}
