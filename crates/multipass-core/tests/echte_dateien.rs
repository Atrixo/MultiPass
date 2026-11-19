mod common;

use common::*;
use multipass_core::{assign_tools, default_inventory, default_tool_map, merge, natural_key, MergeOptions, NcFile};
use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore = "braucht MULTIPASS_REAL_DIR mit echten NC-Dateien und Python"]
fn echte_dateien_gleichen_python_referenz() {
    let dir = PathBuf::from(std::env::var("MULTIPASS_REAL_DIR").expect("MULTIPASS_REAL_DIR setzen"));
    let python = std::env::var("MULTIPASS_PYTHON").unwrap_or_else(|_| "python".into());
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("cnc")))
        .collect();
    paths.sort_by_key(|p| natural_key(&p.file_name().unwrap().to_string_lossy()));
    assert!(!paths.is_empty());
    let out = std::env::temp_dir().join(format!("multipass_real_{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    let files: Vec<NcFile> = paths.iter().map(|p| NcFile::read(p).unwrap()).collect();
    let maps = default_tool_map(&files);
    for restore in [false, true] {
        let py_out = out.join(format!("py_{restore}.cnc"));
        let mut cmd = Command::new(&python);
        cmd.arg("-I").arg(root().join("reference/multipass.py")).arg("--cli");
        cmd.args(&paths).arg("-o").arg(&py_out);
        if restore {
            cmd.arg("--rapid");
        }
        assert!(cmd.status().unwrap().success());
        let py = std::fs::read_to_string(&py_out).unwrap();
        let t = std::time::Instant::now();
        let r = merge(&files, &maps, &MergeOptions { restore, safe_z: 1.0, stamp: "x".into() }).unwrap();
        println!("restore={restore}: {:?}, {:?}", t.elapsed(), r.report.last());
        assert!(without_stamp(&r.text) == without_stamp(&py), "Abweichung bei restore={restore}");
    }
    std::fs::remove_dir_all(&out).ok();

    let inv: Vec<u64> = assign_tools(&files, &default_inventory()).iter().map(|m| m[&1]).collect();
    let from_names: Vec<u64> = files
        .iter()
        .map(|f| f.name.split('_').nth(1).and_then(|t| t.trim_start_matches('T').parse().ok()).unwrap_or(0))
        .collect();
    assert_eq!(inv, from_names, "Inventarnummern passen nicht zu den Dateinamen");
}
