mod common;

use common::*;
use multipass_core::{estimate, preview, restore_rapids, NcFile, Units};

pub fn check_consistency(f: &NcFile) -> (f64, f64) {
    let inch = f.units == Units::Inch;
    let plain = estimate(f.body(), inch);
    let (out, stats) = restore_rapids(f.body(), 1.0).unwrap();
    let fast = estimate(&out, inch);
    let tol = 1e-6 * (1.0 + plain.cut_mm + plain.rapid_mm);
    assert_eq!(plain.moves, fast.moves, "{}", f.name);
    assert!((fast.rapid_mm - (plain.rapid_mm + stats.distance_mm)).abs() < tol, "{}", f.name);
    assert!((fast.cut_mm - (plain.cut_mm - stats.distance_mm)).abs() < tol, "{}", f.name);
    assert!((fast.cut_min - (plain.cut_min - stats.feed_time_min)).abs() < 1e-6 * (1.0 + plain.cut_min), "{}", f.name);
    (plain.total_min(3000.0), fast.total_min(3000.0))
}

#[test]
fn estimate_matches_rapid_statistics() {
    for name in [ENGRAVE, CONTOUR, EDGE_A, EDGE_B] {
        let f = fixture(name);
        let (plain, fast) = check_consistency(&f);
        assert!(plain > 0.0 && fast < plain, "{name}: {plain} -> {fast}");
    }
}

#[test]
fn preview_of_test_files() {
    for name in [ENGRAVE, CONTOUR, EDGE_A] {
        let f = fixture(name);
        let p = preview(f.body(), false, 50_000);
        assert!(p.points > 10, "{name}");
        assert!(p.bounds.is_some());
        assert!(p.polylines.iter().all(|l| l.points.len() % 2 == 0 && l.points.len() >= 4));
    }
}
