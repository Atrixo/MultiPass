use crate::job::effective_body;
use crate::motion::{simulate, Move, MoveKind};
use crate::ncfile::{NcFile, Units};
use crate::pycompat::{parse_float, regex};
use regex::Regex;
use serde::Serialize;
use std::borrow::Borrow;
use std::sync::LazyLock;

static TOOL_LINE: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)^\(\s*T(\d+)\s.*\bD=([-+\d.]+)"));
static FIELD: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)\b(TD|CR|TAPER)=([-+\d.]+)"));
static STOCK_HEIGHT: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)Stock\s+Height\s*:\s*([\d.]+)"));

pub const STRIDE: usize = 5;
const CHORD_TOL: f64 = 0.01;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolShape {
    pub diameter: f64,
    pub tip: f64,
    pub corner: f64,
    pub taper_deg: f64,
}

impl ToolShape {
    pub fn flat(diameter: f64) -> ToolShape {
        ToolShape { diameter, tip: diameter, corner: 0.0, taper_deg: 0.0 }
    }

    pub fn height_at(&self, d: f64) -> Option<f64> {
        let r = self.diameter / 2.0;
        if d > r {
            return None;
        }
        if self.taper_deg > 0.0 {
            let tip = (self.tip / 2.0).min(r);
            return Some(if d <= tip { 0.0 } else { (d - tip) / self.taper_deg.to_radians().tan() });
        }
        let c = self.corner.clamp(0.0, r);
        let flat = r - c;
        if d <= flat || c <= 0.0 {
            return Some(0.0);
        }
        let e = d - flat;
        Some(c - (c * c - e * e).max(0.0).sqrt())
    }
}

pub fn tool_shapes(f: &NcFile) -> Vec<(u64, ToolShape)> {
    let scale = if f.units == Units::Inch { 25.4 } else { 1.0 };
    let mut out: Vec<(u64, ToolShape)> = Vec::new();
    for line in &f.lines[..f.body_start] {
        let Some(c) = TOOL_LINE.captures(line) else { continue };
        let (Ok(n), Some(d)) = (c[1].parse::<u64>(), parse_float(&c[2])) else { continue };
        if d <= 0.0 || out.iter().any(|(m, _)| *m == n) {
            continue;
        }
        let mut s = ToolShape::flat(d * scale);
        for fc in FIELD.captures_iter(line) {
            let Some(v) = parse_float(&fc[2]) else { continue };
            match fc[1].to_ascii_uppercase().as_str() {
                "TD" => s.tip = (v * scale).max(0.0),
                "CR" => s.corner = (v * scale).max(0.0),
                _ => s.taper_deg = v.clamp(0.0, 89.0),
            }
        }
        out.push((n, s));
    }
    out
}

pub fn stock_height(f: &NcFile) -> Option<f64> {
    let scale = if f.units == Units::Inch { 25.4 } else { 1.0 };
    f.lines[..f.body_start]
        .iter()
        .find_map(|l| STOCK_HEIGHT.captures(l).and_then(|c| parse_float(&c[1])))
        .map(|v| v * scale)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimTool {
    pub part: usize,
    pub tool: Option<u64>,
    pub shape: ToolShape,
    pub known: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimPart {
    pub name: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineMeta {
    pub parts: Vec<SimPart>,
    pub tools: Vec<SimTool>,
    pub bounds: Option<[f64; 4]>,
    pub material: bool,
    pub stock_height: Option<f64>,
    pub zmin: f64,
    pub vertices: usize,
}

#[derive(Debug, Clone)]
pub struct Timeline {
    pub meta: TimelineMeta,
    pub data: Vec<f32>,
}

struct Builder {
    data: Vec<f32>,
    last: Option<[f64; 3]>,
}

impl Builder {
    fn push(&mut self, p: [f64; 3], dur: f64, slot: usize, rapid: bool) {
        let code = slot as f64 * 2.0 + if rapid { 1.0 } else { 0.0 };
        self.data.extend([p[0] as f32, p[1] as f32, p[2] as f32, dur as f32, code as f32]);
        self.last = Some(p);
    }

    fn jump(&mut self, p: [f64; 3], slot: usize) {
        self.push(p, f64::NAN, slot, true);
    }
}

fn steps(m: &Move) -> usize {
    match m.arc {
        Some(a) => {
            let r = a.radius.max(1e-6);
            let ang = 2.0 * (1.0 - (CHORD_TOL / r).min(1.0)).acos();
            ((a.sweep.abs() / ang.max(1e-3)).ceil() as usize).clamp(1, 720)
        }
        None => 1,
    }
}

fn slot_of(tools: &mut Vec<SimTool>, shapes: &[(u64, ToolShape)], part: usize, tool: Option<u64>) -> usize {
    if let Some(i) = tools.iter().position(|t| t.part == part && t.tool == tool) {
        return i;
    }
    let found = tool.and_then(|n| shapes.iter().find(|(m, _)| *m == n)).or(shapes.first());
    tools.push(SimTool {
        part,
        tool,
        shape: found.map(|(_, s)| *s).unwrap_or(ToolShape::flat(3.175)),
        known: found.is_some(),
    });
    tools.len() - 1
}

fn far(a: [f64; 3], b: [f64; 3]) -> bool {
    (a[0] - b[0]).abs() > 1e-6 || (a[1] - b[1]).abs() > 1e-6 || (a[2] - b[2]).abs() > 1e-6
}

pub fn timeline<F: Borrow<NcFile>>(files: &[F], safe_z: Option<f64>) -> Timeline {
    let mut b = Builder { data: Vec::new(), last: None };
    let mut parts = Vec::new();
    let mut tools: Vec<SimTool> = Vec::new();
    let mut bounds: Option<[f64; 4]> = None;
    let mut zmin = 0.0f64;
    let material =
        !files.is_empty() && files.iter().map(Borrow::borrow).all(|f: &NcFile| f.is_stock_top() && !f.incremental);
    let mut stock = None;
    for (pi, f) in files.iter().map(Borrow::borrow).enumerate() {
        let shapes = tool_shapes(f);
        stock = stock.or_else(|| stock_height(f));
        let start = b.data.len() / STRIDE;
        let sim = simulate(&effective_body(f, safe_z), f.units == Units::Inch);
        let mut fresh = true;
        for m in &sim.moves {
            let slot = slot_of(&mut tools, &shapes, pi, m.tool);
            let rapid = m.kind == MoveKind::Rapid;
            if fresh || b.last.is_none_or(|l| far(l, m.from)) {
                b.jump(m.from, slot);
                fresh = false;
            }
            let minutes = match (rapid, m.feed.filter(|f| *f > 0.0)) {
                (false, Some(feed)) => Some(m.length / feed),
                _ => None,
            };
            let n = steps(m);
            for i in 1..=n {
                let t = i as f64 / n as f64;
                let xy = match m.arc {
                    Some(a) if i < n => a.point(t),
                    _ => [m.to[0], m.to[1]],
                };
                let p = [xy[0], xy[1], m.from[2] + (m.to[2] - m.from[2]) * t];
                let dur = match minutes {
                    Some(min) => min / n as f64,
                    None => -(m.length / n as f64),
                };
                b.push(p, dur, slot, rapid);
                if !rapid && p[2] < 0.0 {
                    let r = tools[slot].shape.diameter / 2.0;
                    let bb = bounds.get_or_insert([p[0], p[1], p[0], p[1]]);
                    bb[0] = bb[0].min(p[0] - r);
                    bb[1] = bb[1].min(p[1] - r);
                    bb[2] = bb[2].max(p[0] + r);
                    bb[3] = bb[3].max(p[1] + r);
                    zmin = zmin.min(p[2]);
                }
            }
        }
        parts.push(SimPart { name: f.name.clone(), start, end: b.data.len() / STRIDE });
    }
    let vertices = b.data.len() / STRIDE;
    Timeline {
        meta: TimelineMeta { parts, tools, bounds, material, stock_height: stock, zmin, vertices },
        data: b.data,
    }
}

pub fn encode(t: &Timeline) -> Vec<u8> {
    let json = serde_json::to_vec(&t.meta).unwrap_or_else(|_| b"{}".to_vec());
    let pad = (4 - (4 + json.len()) % 4) % 4;
    let mut out = Vec::with_capacity(4 + json.len() + pad + t.data.len() * 4);
    out.extend((json.len() as u32).to_le_bytes());
    out.extend(&json);
    out.extend(std::iter::repeat_n(b' ', pad));
    for v in &t.data {
        out.extend(v.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "(T1  Single Flute Engraving Metal 30 deg*.3mm      D=3.175 SD=3.175 TD=0.3 FL=9. SL=9. BL=19. TAPER=15deg - ZMIN=-2.1 - Fasenfraser)\n(   Z Origin Set To   : Stock Top )\n(   Stock Height                    : 5. mm )\nG90 G21\n";

    fn file(body: &str) -> NcFile {
        NcFile::parse("a.cnc", format!("{HEAD}(Op)\nT1M6\n{body}M30\n").as_bytes()).unwrap()
    }

    #[test]
    fn werkzeugform_aus_kopfzeile() {
        let f = file("");
        let s = tool_shapes(&f);
        assert_eq!(s.len(), 1);
        let v = s[0].1;
        assert!((v.diameter - 3.175).abs() < 1e-9 && (v.tip - 0.3).abs() < 1e-9 && v.taper_deg == 15.0);
        assert_eq!(v.height_at(0.1), Some(0.0));
        assert!((v.height_at(0.15 + 15f64.to_radians().tan()).unwrap() - 1.0).abs() < 1e-9);
        assert_eq!(v.height_at(2.0), None);
        let ball = ToolShape { diameter: 1.5, tip: 0.0, corner: 0.75, taper_deg: 0.0 };
        assert_eq!(ball.height_at(0.0), Some(0.0));
        assert!((ball.height_at(0.75).unwrap() - 0.75).abs() < 1e-9);
        assert_eq!(ToolShape::flat(2.0).height_at(1.0), Some(0.0));
        assert_eq!(stock_height(&f), Some(5.0));
    }

    #[test]
    fn zeitleiste_mit_boegen_spruengen_und_dauer() {
        let f = file("G0 X0 Y0 Z5\nG1 Z-1 F60\nG2 X10 Y0 I5 J0 F120\nG0 Z5\nG28\nG0 X20 Y0 Z5\nX30\n");
        let t = timeline(&[&f], None);
        let d = &t.data;
        let n = d.len() / STRIDE;
        assert_eq!(n, t.meta.vertices);
        assert!(d[3].is_nan());
        let cut: f64 = (0..n).map(|i| d[i * STRIDE + 3] as f64).filter(|v| *v > 0.0).sum();
        let expect = 6.0 / 60.0 + (std::f64::consts::PI * 5.0) / 120.0;
        assert!((cut - expect).abs() < 1e-4, "{cut} {expect}");
        let jumps = (0..n).filter(|i| d[i * STRIDE + 3].is_nan()).count();
        assert_eq!(jumps, 2);
        let top = (0..n).map(|i| d[i * STRIDE + 1]).fold(f32::NEG_INFINITY, f32::max);
        assert!((top - 5.0).abs() < 0.02, "{top}");
        let b = t.meta.bounds.unwrap();
        assert!(b[0] < -1.5 && b[2] > 11.5);
        assert!(t.meta.material);
        assert_eq!(t.meta.zmin, -1.0);
        assert_eq!(t.meta.tools.len(), 1);
        assert!(t.meta.tools[0].known);
        let bytes = encode(&t);
        let len = u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
        let offset = (4 + len).div_ceil(4) * 4;
        assert_eq!(bytes.len() - offset, d.len() * 4);
        assert!(serde_json::from_slice::<serde_json::Value>(&bytes[4..4 + len]).is_ok());
    }

    #[test]
    fn eilgang_und_unbekannter_vorschub_als_strecke() {
        let f = file("G0 X0 Y0 Z5\nX10\nG1 X20\n");
        let t = timeline(&[&f], None);
        let durs: Vec<f32> = t.data.chunks(STRIDE).map(|c| c[3]).collect();
        assert!(durs[0].is_nan());
        assert_eq!(&durs[1..], &[-10.0, -10.0]);
        let codes: Vec<f32> = t.data.chunks(STRIDE).map(|c| c[4]).collect();
        assert_eq!(&codes[1..], &[1.0, 0.0]);
    }
}
