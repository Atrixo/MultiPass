use crate::ncfile::COMMENT;
use crate::pycompat::{fmt_num, lstrip, parse_float, regex, rstrip, strip};
use regex::Regex;
use serde::Serialize;
use std::borrow::Cow;
use std::sync::LazyLock;

static WORD: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)([A-Z])\s*([-+]?(?:\d+\.?\d*|\.\d+))"));

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RapidStats {
    pub converted: usize,
    pub distance_mm: f64,
    pub feed_time_min: f64,
}

impl RapidStats {
    pub fn add(&mut self, o: &RapidStats) {
        self.converted += o.converted;
        self.distance_mm += o.distance_mm;
        self.feed_time_min += o.feed_time_min;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonAsciiCode {
    pub line: usize,
}

const AXES: [char; 3] = ['X', 'Y', 'Z'];

fn axis(c: char) -> Option<usize> {
    AXES.iter().position(|&a| a == c)
}

fn coord_text(v: f64) -> String {
    let s = fmt_num(v);
    if s.parse::<f64>().ok() == Some(v) {
        s
    } else {
        format!("{v}")
    }
}

fn semicolon_comment(line: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in line.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ';' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

fn append_feed(line: &str, feed: f64) -> String {
    match semicolon_comment(line) {
        Some(p) => format!("{} F{} {}", rstrip(&line[..p]), fmt_num(feed), &line[p..]),
        None => format!("{} F{}", rstrip(line), fmt_num(feed)),
    }
}

fn truthy(f: Option<f64>) -> Option<f64> {
    f.filter(|v| *v != 0.0)
}

pub fn restore_rapids<'a, S: AsRef<str>>(
    lines: &'a [S],
    safe_z: f64,
) -> Result<(Vec<Cow<'a, str>>, RapidStats), NonAsciiCode> {
    let mut out: Vec<Cow<'a, str>> = Vec::with_capacity(lines.len());
    let mut mode_in: Option<i32> = None;
    let mut mode_out: Option<i32> = None;
    let mut pos: [Option<f64>; 3] = [None; 3];
    let mut feed: Option<f64> = None;
    let mut feed_dirty = false;
    let mut stats = RapidStats::default();

    for (idx, line) in lines.iter().enumerate() {
        let line = line.as_ref();
        let without_comment = COMMENT.replace_all(line, "");
        let code = strip(&without_comment);
        if code.is_empty() {
            out.push(Cow::Borrowed(line));
            continue;
        }
        if !code.is_ascii() {
            return Err(NonAsciiCode { line: idx });
        }

        let mut letters_ok = true;
        let mut gvals: Vec<f64> = Vec::new();
        let mut fvals: Vec<f64> = Vec::new();
        let mut coords: [Option<f64>; 3] = [None; 3];
        for c in WORD.captures_iter(code) {
            let letter = c[1].chars().next().unwrap_or(' ').to_ascii_uppercase();
            let v = match parse_float(&c[2]) {
                Some(v) => v,
                None => return Err(NonAsciiCode { line: idx }),
            };
            if !matches!(letter, 'G' | 'X' | 'Y' | 'Z' | 'F') {
                letters_ok = false;
            }
            match letter {
                'G' => gvals.push(v),
                'F' => fvals.push(v),
                _ => {
                    if let Some(a) = axis(letter) {
                        coords[a] = Some(v);
                    }
                }
            }
        }
        let has_coords = coords.iter().any(Option::is_some);
        let motion_g: Vec<f64> = gvals.iter().copied().filter(|g| [0.0, 1.0, 2.0, 3.0].contains(g)).collect();
        let only_motion = letters_ok && gvals.iter().all(|g| *g == 0.0 || *g == 1.0) && gvals.len() <= 1;
        let new_mode = match motion_g.last() {
            Some(g) => Some(*g as i32),
            None => mode_in,
        };
        if let Some(f) = fvals.last() {
            feed = Some(*f);
        }

        if has_coords && only_motion && new_mode == Some(1) {
            if let [Some(sx), Some(sy), Some(sz)] = pos {
                let ex = coords[0].unwrap_or(sx);
                let ey = coords[1].unwrap_or(sy);
                let ez = coords[2].unwrap_or(sz);
                let (dx, dy, dz) = (ex - sx, ey - sy, ez - sz);
                let length = (dx * dx + dy * dy + dz * dz).sqrt();
                let pure_up = dx.abs() < 1e-9 && dy.abs() < 1e-9 && dz > 0.0;
                let in_air = sz >= safe_z && ez >= safe_z;
                if (pure_up || in_air) && length > 0.0 {
                    let end = [ex, ey, ez];
                    let mut s = String::from("G0");
                    for (a, name) in AXES.iter().enumerate() {
                        if coords[a].is_some() {
                            s.push(' ');
                            s.push(*name);
                            s.push_str(&coord_text(end[a]));
                        }
                    }
                    out.push(Cow::Owned(s));
                    mode_out = Some(0);
                    mode_in = Some(1);
                    if !fvals.is_empty() {
                        feed_dirty = true;
                    }
                    stats.converted += 1;
                    stats.distance_mm += length;
                    if let Some(f) = truthy(feed) {
                        stats.feed_time_min += length / f;
                    }
                    pos = [Some(ex), Some(ey), Some(ez)];
                    continue;
                }
            }
        }

        let cutting = matches!(new_mode, Some(1..=3));
        let mut l: Cow<'a, str> = Cow::Borrowed(line);
        if has_coords && cutting && mode_out == Some(0) && motion_g.is_empty() {
            l = Cow::Owned(format!("G{} {}", new_mode.unwrap_or(1), lstrip(line)));
        }
        if has_coords && cutting {
            if feed_dirty && fvals.is_empty() {
                if let Some(f) = feed {
                    l = Cow::Owned(append_feed(&l, f));
                }
            }
            feed_dirty = false;
        }
        out.push(l);
        if new_mode.is_some() && (has_coords || !motion_g.is_empty()) {
            mode_out = new_mode;
        }
        mode_in = new_mode;
        for a in 0..3 {
            if coords[a].is_some() {
                pos[a] = coords[a];
            }
        }
        if gvals.iter().any(|g| *g == 28.0 || *g == 30.0 || *g == 53.0) {
            pos = [None; 3];
        }
    }
    Ok((out, stats))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str) -> Vec<String> {
        let lines: Vec<&str> = src.lines().collect();
        restore_rapids(&lines, 1.0).unwrap().0.into_iter().map(Cow::into_owned).collect()
    }

    #[test]
    fn leerfahrt_ueber_material_wird_eilgang() {
        let out = run("G0 X0 Y0 Z10\nG1 X5 Y5 F500\nG1 Z0.5\nX6");
        assert_eq!(out, vec!["G0 X0 Y0 Z10", "G0 X5. Y5.", "G1 Z0.5 F500.", "X6"]);
    }

    #[test]
    fn eintauchen_und_material_bleiben() {
        let out = run("G0 X0 Y0 Z0.5\nG1 X5 F100\nG1 Z-1\nG1 Z0.9");
        assert_eq!(out, vec!["G0 X0 Y0 Z0.5", "G1 X5 F100", "G1 Z-1", "G0 Z0.9"]);
    }

    #[test]
    fn boegen_werden_nie_umgewandelt() {
        let out = run("G0 X0 Y0 Z10\nG2 X5 Y5 I2.5 J2.5 F100\nG3 X0 Y0 R5");
        assert_eq!(out, vec!["G0 X0 Y0 Z10", "G2 X5 Y5 I2.5 J2.5 F100", "G3 X0 Y0 R5"]);
    }

    #[test]
    fn modus_nach_umwandlung_wieder_gesetzt() {
        let out = run("G0 X0 Y0 Z10\nG1 X1 F100\nX2\nZ0.5");
        assert_eq!(out, vec!["G0 X0 Y0 Z10", "G0 X1.", "G0 X2.", "G1 Z0.5 F100."]);
    }

    #[test]
    fn position_unbekannt_nach_g28() {
        let out = run("G0 X0 Y0 Z10\nG28\nG1 X5 Y5 F100\nZ20\nX7 Y7");
        assert_eq!(out, vec!["G0 X0 Y0 Z10", "G28", "G1 X5 Y5 F100", "Z20", "G0 X7. Y7."]);
        let out = run("G0 X0 Y0 Z10\nG1 X1 Y1 Z12 F100\nG53 Z0\nZ20\nX7 Y7");
        assert_eq!(out[2], "G1 G53 Z0 F100.");
        assert_eq!(&out[3..], &["Z20", "X7 Y7"]);
    }

    #[test]
    fn andere_woerter_verhindern_umwandlung() {
        let out = run("G0 X0 Y0 Z10\nN10 G1 X5 F100\nG1 X6 S1000\nG1 G90 X7");
        assert_eq!(out, vec!["G0 X0 Y0 Z10", "N10 G1 X5 F100", "G1 X6 S1000", "G1 G90 X7"]);
    }

    #[test]
    fn mehr_als_vier_nachkommastellen_bleiben_exakt() {
        let out = run("G0 X0 Y0 Z10\nG1 X.00005 Y-0.00004 F100");
        assert_eq!(out[1], "G0 X0.00005 Y-0.00004");
        let out = run("G0 X0 Y0 Z10\nG1 X1.5 Y-2. F100");
        assert_eq!(out[1], "G0 X1.5 Y-2.");
    }

    #[test]
    fn vorschub_wird_vor_semikolon_kommentar_eingefuegt() {
        let out = run("G0 X0 Y0 Z10\nG1 X5 F900\nZ-1 ; eintauchen (tief)\nX6 (a;b)");
        assert_eq!(out[2], "G1 Z-1 F900. ; eintauchen (tief)");
        let out = run("G0 X0 Y0 Z10\nG1 X5 F900\nZ-1 (a;b)");
        assert_eq!(out[2], "G1 Z-1 (a;b) F900.");
    }

    #[test]
    fn nicht_ascii_im_code_bricht_ab() {
        let lines = ["G0 X0 Y0 Z10", "(Öffnung)", "G1 X\u{0661} F100"];
        assert_eq!(restore_rapids(&lines, 1.0).unwrap_err(), NonAsciiCode { line: 2 });
    }

    #[test]
    fn statistik() {
        let lines = ["G0 X0 Y0 Z10", "G1 X30 Y40 F100"];
        let (_, s) = restore_rapids(&lines, 1.0).unwrap();
        assert_eq!(s.converted, 1);
        assert_eq!(s.distance_mm, 50.0);
        assert_eq!(s.feed_time_min, 0.5);
    }
}
