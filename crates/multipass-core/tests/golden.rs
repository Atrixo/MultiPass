mod common;

use common::*;
use multipass_core::{default_tool_map, merge, MergeOptions, NcFile};

#[test]
fn output_matches_python_reference() {
    for case in CASES {
        let files: Vec<NcFile> = case.files.iter().map(|f| fixture(f)).collect();
        let opts = MergeOptions {
            restore: case.restore,
            safe_z: case.safe_z,
            stamp: "2026-01-01 00:00".into(),
            fan_pause: false,
        };
        let r = merge(&files, &default_tool_map(&files), &opts).unwrap();
        let golden = std::fs::read(root().join("tests/golden").join(case.golden)).unwrap();
        let golden = String::from_utf8(golden).unwrap();
        let golden = english_comments(&golden);
        let (a, b) = (without_stamp(&r.text), without_stamp(&golden));
        if a != b {
            let first = a.lines().zip(b.lines()).position(|(x, y)| x != y);
            panic!(
                "{}: differs (line {:?})\nRust:   {:?}\nPython: {:?}",
                case.golden,
                first.map(|i| i + 2),
                first.and_then(|i| a.lines().nth(i)),
                first.and_then(|i| b.lines().nth(i))
            );
        }
        if case.restore {
            assert!(r.rapid.is_some_and(|s| s.converted > 0), "{}", case.golden);
        }
    }
}

#[test]
fn merged_file_can_be_read_again() {
    let files = vec![fixture(ENGRAVE), fixture(CONTOUR)];
    let opts = MergeOptions { restore: true, safe_z: 1.0, stamp: "x".into(), fan_pause: false };
    let r = merge(&files, &default_tool_map(&files), &opts).unwrap();
    let again = NcFile::parse("GESAMT.cnc", r.text.as_bytes()).unwrap();
    assert_eq!(again.tool_numbers, vec![1, 2]);
    assert_eq!(again.newline, "\r\n");
    assert!(r.text.is_ascii());
    assert_eq!(r.text.lines().filter(|l| l.trim() == "M30").count(), 1);
    assert_eq!(r.text.lines().last(), Some("M30"));
}
