use multipass_core::{default_tool_map, merge, MergeOptions, NcFile};
use std::fmt::Write;
use std::time::Instant;

fn synthetic(lines: usize) -> String {
    let mut s = String::with_capacity(lines * 24);
    for l in [
        "(gross)",
        "(T1  Spiral O Metal 3.175*12mm  Makera    D=3.175)",
        "(   Z Origin Set To   : Stock Top )",
        "G90 G94",
        "G21",
        "(op)",
        "T1M6",
        "S12000 M3",
        "G0 X0 Y0",
        "Z15.",
    ] {
        s.push_str(l);
        s.push_str("\r\n");
    }
    let mut i = 0;
    while i < lines {
        let x = (i % 997) as f64 * 0.137;
        let y = (i % 991) as f64 * 0.071;
        let _ = write!(
            s,
            "G1 Z5. F100\r\nX{x:.3} Y{y:.3} F800\r\nG1 Z-0.5 F100\r\nX{:.3} Y{y:.3} F600\r\nG2 X{:.3} Y{:.3} I0.5 J0.\r\nG1 Z15.\r\n",
            x + 1.0,
            x + 1.5,
            y + 0.5
        );
        i += 6;
    }
    s.push_str("M5\r\nG28\r\nM30\r\n");
    s
}

#[test]
fn eine_million_zeilen() {
    let src = synthetic(1_000_000);
    let t = Instant::now();
    let files = vec![NcFile::parse("a.cnc", src.as_bytes()).unwrap()];
    let maps = default_tool_map(&files);
    let r = merge(&files, &maps, &MergeOptions { restore: true, safe_z: 1.0, stamp: "x".into() }).unwrap();
    let dauer = t.elapsed();
    let n = r.rapid.unwrap().converted;
    println!("1 Mio. Zeilen: {dauer:?}, {n} umgewandelt");
    assert!(n > 300_000);
    if !cfg!(debug_assertions) {
        assert!(dauer.as_secs_f64() < 3.0, "zu langsam: {dauer:?}");
    }
}
