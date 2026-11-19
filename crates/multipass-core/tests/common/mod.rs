#![allow(dead_code)]

use multipass_core::NcFile;
use std::path::{Path, PathBuf};

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn fixture(rel: &str) -> NcFile {
    NcFile::read(&root().join("tests/fixtures").join(rel)).unwrap()
}

pub const GRAVUR: &str = "4_T16_Gravur.cnc";
pub const KONTUR: &str = "5_T13_Kontur.cnc";
pub const EDGE_A: &str = "edge/a_mehrwerkzeug.nc";
pub const EDGE_B: &str = "edge/b_\u{dc}bung (Test).nc";

pub struct Case {
    pub golden: &'static str,
    pub files: &'static [&'static str],
    pub restore: bool,
    pub safe_z: f64,
}

pub const CASES: &[Case] = &[
    Case { golden: "g01_gravur_kontur.cnc", files: &[GRAVUR, KONTUR], restore: false, safe_z: 1.0 },
    Case { golden: "g02_gravur_kontur_rapid.cnc", files: &[GRAVUR, KONTUR], restore: true, safe_z: 1.0 },
    Case { golden: "g03_kontur_gravur_kontur_rapid.cnc", files: &[KONTUR, GRAVUR, KONTUR], restore: true, safe_z: 1.0 },
    Case { golden: "g04_edge_ab.nc", files: &[EDGE_A, EDGE_B], restore: false, safe_z: 1.0 },
    Case { golden: "g05_edge_ab_rapid.nc", files: &[EDGE_A, EDGE_B], restore: true, safe_z: 1.0 },
    Case { golden: "g06_edge_ba_rapid.nc", files: &[EDGE_B, EDGE_A], restore: true, safe_z: 1.0 },
    Case { golden: "g07_edge_a_gravur_rapid.nc", files: &[EDGE_A, GRAVUR], restore: true, safe_z: 1.0 },
    Case { golden: "g08_edge_ab_rapid_safez5.nc", files: &[EDGE_A, EDGE_B], restore: true, safe_z: 5.0 },
];

pub fn without_stamp(text: &str) -> &str {
    let first = text.split_inclusive('\n').next().unwrap_or("");
    assert!(first.starts_with("(Zusammengefuegt mit MultiPass "), "Kopfzeile fehlt: {first}");
    &text[first.len()..]
}

pub struct Move {
    pub mode: Option<i32>,
    pub feed: Option<f64>,
    pub pos: [Option<f64>; 3],
    pub start: [Option<f64>; 3],
}

pub fn simulate(text: &str) -> Vec<Move> {
    let word = regex::Regex::new(r"([A-Z])\s*([-+]?(?:\d+\.?\d*|\.\d+))").unwrap();
    let comment = regex::Regex::new(r"\([^)]*\)|;.*$").unwrap();
    let mut mode = None;
    let mut feed = None;
    let mut pos: [Option<f64>; 3] = [None; 3];
    let mut res = Vec::new();
    for line in text.lines() {
        let code = comment.replace_all(line, "").trim().to_uppercase();
        if code.is_empty() {
            continue;
        }
        let w: Vec<(char, f64)> =
            word.captures_iter(&code).map(|c| (c[1].chars().next().unwrap(), c[2].parse().unwrap())).collect();
        let g: Vec<f64> = w.iter().filter(|(a, _)| *a == 'G').map(|(_, v)| *v).collect();
        for v in &g {
            if [0.0, 1.0, 2.0, 3.0].contains(v) {
                mode = Some(*v as i32);
            }
        }
        for (a, v) in &w {
            if *a == 'F' {
                feed = Some(*v);
            }
        }
        let reset = g.iter().any(|x| [28.0, 30.0, 53.0].contains(x));
        let co: Vec<(usize, f64)> = w.iter().filter_map(|(a, v)| "XYZ".find(*a).map(|i| (i, *v))).collect();
        if !co.is_empty() && !reset {
            let start = pos;
            for (i, v) in co {
                pos[i] = Some(v);
            }
            let f = if matches!(mode, Some(1..=3)) { feed } else { None };
            res.push(Move { mode, feed: f, pos, start });
        }
        if reset {
            pos = [None; 3];
        }
    }
    res
}
