mod common;

use common::*;
use multipass_core::{default_tool_map, merge, restore_rapids, MergeOptions, NcFile};

fn check(plain: &str, fast: &str, safe_z: f64) -> usize {
    assert_eq!(plain.lines().count(), fast.lines().count(), "Zeilenzahl veraendert");
    let (a, b) = (simulate(plain), simulate(fast));
    assert_eq!(a.len(), b.len(), "Anzahl Bewegungen veraendert");
    let mut converted = 0;
    for (i, (x, y)) in a.iter().zip(&b).enumerate() {
        assert_eq!(x.pos, y.pos, "Position veraendert (Bewegung {i})");
        if x.mode != y.mode {
            converted += 1;
            assert_eq!((x.mode, y.mode), (Some(1), Some(0)), "falsche Umwandlung (Bewegung {i})");
            let s = x.start.map(|v| v.expect("Umwandlung bei unbekannter Position"));
            let e = x.pos.map(|v| v.unwrap());
            let in_air = s[2] >= safe_z && e[2] >= safe_z;
            let pure_up = (s[0] - e[0]).abs() < 1e-9 && (s[1] - e[1]).abs() < 1e-9 && e[2] > s[2];
            assert!(in_air || pure_up, "Eilgang zu tief (Bewegung {i}): {s:?} -> {e:?}");
        } else if matches!(x.mode, Some(1..=3)) {
            assert_eq!(x.feed, y.feed, "Vorschub veraendert (Bewegung {i})");
        }
    }
    converted
}

#[test]
fn eilgang_ist_sicher_fuer_alle_testdateien() {
    for case in CASES.iter().filter(|c| c.restore) {
        let files: Vec<NcFile> = case.files.iter().map(|f| fixture(f)).collect();
        let maps = default_tool_map(&files);
        let mk = |restore| MergeOptions { restore, safe_z: case.safe_z, stamp: "x".into() };
        let plain = merge(&files, &maps, &mk(false)).unwrap();
        let fast = merge(&files, &maps, &mk(true)).unwrap();
        let n = check(&plain.text, &fast.text, case.safe_z);
        assert!(n > 0, "{}", case.golden);
        assert_eq!(Some(n), fast.rapid.map(|s| s.converted), "{}", case.golden);
    }
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn num(&mut self) -> String {
        let v = self.below(4000) as f64 / 100.0 - 10.0;
        match self.below(6) {
            0 => format!("{:.5}", v + self.below(10) as f64 * 0.00001),
            1 => format!("{v:.0}."),
            2 => format!("{v}"),
            _ => format!("{v:.3}"),
        }
    }
}

fn random_program(rng: &mut Rng, len: usize) -> Vec<String> {
    let mut out = vec!["G0 X0 Y0 Z10".to_string()];
    for _ in 0..len {
        let mut l = String::new();
        match rng.below(20) {
            0 => {
                out.push("G28".into());
                continue;
            }
            1 => {
                out.push(format!("G53 Z{}", rng.num()));
                continue;
            }
            2 => {
                out.push("M5 (stop)".into());
                continue;
            }
            3 => l.push_str("N10 "),
            4 => l.push_str("G0 "),
            5 | 6 => l.push_str("G1 "),
            7 => l.push_str(if rng.below(2) == 0 { "G2 " } else { "G3 " }),
            _ => {}
        }
        let axes = rng.below(7) + 1;
        if axes & 1 != 0 {
            l.push_str(&format!("X{} ", rng.num()));
        }
        if axes & 2 != 0 {
            l.push_str(&format!("Y{} ", rng.num()));
        }
        if axes & 4 != 0 {
            l.push_str(&format!("Z{} ", rng.num()));
        }
        if rng.below(4) == 0 {
            l.push_str(&format!("F{}", rng.below(2000)));
        }
        match rng.below(12) {
            0 => l.push_str(" ; kommentar"),
            1 => l.push_str(" (a;b)"),
            2 => l.insert_str(0, "(vorne) "),
            _ => {}
        }
        let l = l.trim_end().to_string();
        out.push(if rng.below(15) == 0 { l.to_lowercase() } else { l });
    }
    out
}

#[test]
fn eilgang_ist_sicher_fuer_zufallsprogramme() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut total = 0;
    let runs: usize = std::env::var("MULTIPASS_FUZZ").ok().and_then(|v| v.parse().ok()).unwrap_or(3000);
    for _ in 0..runs {
        let prog = random_program(&mut rng, 60);
        let safe_z = [0.5, 1.0, 2.5, 20.0][rng.below(4) as usize];
        let (out, stats) = restore_rapids(&prog, safe_z).unwrap();
        let plain = prog.join("\n");
        let fast = out.iter().map(|c| c.as_ref()).collect::<Vec<_>>().join("\n");
        let n = match std::panic::catch_unwind(|| check(&plain, &fast, safe_z)) {
            Ok(n) => n,
            Err(e) => {
                for (a, b) in plain.lines().zip(fast.lines()) {
                    eprintln!("{a:<40} | {b}");
                }
                std::panic::resume_unwind(e);
            }
        };
        assert_eq!(n, stats.converted);
        total += n;
    }
    assert!(total > 1000, "zu wenige Umwandlungen geprueft: {total}");
}
