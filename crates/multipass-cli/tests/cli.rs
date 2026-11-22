use multipass_core::{assign_tools, default_inventory, merge, InventoryEntry, MergeOptions, NcFile};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const STAMP: &str = "2026-11-20 08:00";
const ENGRAVE: &str = "4_T16_Gravur.cnc";
const CONTOUR: &str = "5_T13_Kontur.cnc";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Dir {
        let d = std::env::temp_dir().join(format!("multipass_cli_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        for f in [ENGRAVE, CONTOUR] {
            std::fs::copy(fixtures().join(f), d.join(f)).unwrap();
        }
        Dir(d)
    }

    fn p(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn cli(dir: &Dir, settings: Option<&Path>, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_multipass-cli"))
        .current_dir(&dir.0)
        .env("MULTIPASS_SETTINGS", settings.map(Path::to_path_buf).unwrap_or_else(|| dir.p("no-settings.json")))
        .args(args)
        .output()
        .unwrap()
}

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

fn expected(dir: &Dir, names: &[&str], inventory: &[InventoryEntry], restore: bool, fan_pause: bool) -> String {
    let files: Vec<NcFile> = names.iter().map(|n| NcFile::read(&dir.p(n)).unwrap()).collect();
    let maps = assign_tools(&files, inventory);
    merge(&files, &maps, &MergeOptions { restore, safe_z: 1.0, stamp: STAMP.into(), fan_pause }).unwrap().text
}

#[test]
fn same_result_as_the_gui() {
    let d = Dir::new("same");
    let o = cli(&d, None, &[CONTOUR, ENGRAVE, "--sort", "--stamp", STAMP]);
    assert!(o.status.success(), "{}", text(&o));
    let out = std::fs::read_to_string(d.p("MERGED.cnc")).unwrap();
    assert_eq!(out, expected(&d, &[ENGRAVE, CONTOUR], &default_inventory(), false, false));
    assert!(out.contains("\r\nT16M6 \r\n") && out.contains("\r\nT13M6 \r\n"));
    let t = text(&o);
    assert!(
        t.contains("Estimated run time")
            && t.contains("Tool changes: 1 (plus inserting the first tool at the start; the machine asks 2 times)")
            && t.contains("Saved:")
    );
    assert!(!d.p("MERGED.cnc.multipass-tmp").exists());
}

#[test]
fn rapid_only_on_request() {
    let d = Dir::new("rapid");
    let o = cli(&d, None, &[ENGRAVE, CONTOUR, "--rapid", "--stamp", STAMP, "-o", "R.cnc"]);
    assert!(o.status.success(), "{}", text(&o));
    assert_eq!(
        std::fs::read_to_string(d.p("R.cnc")).unwrap(),
        expected(&d, &[ENGRAVE, CONTOUR], &default_inventory(), true, false)
    );
    assert!(text(&o).contains("moves converted") && text(&o).contains("with rapids"));
}

#[test]
fn fan_pause_only_on_request() {
    let d = Dir::new("fan");
    let o = cli(&d, None, &[ENGRAVE, CONTOUR, "--fan-pause", "--stamp", STAMP, "-o", "F.cnc"]);
    assert!(o.status.success(), "{}", text(&o));
    let f = std::fs::read_to_string(d.p("F.cnc")).unwrap();
    assert_eq!(f, expected(&d, &[ENGRAVE, CONTOUR], &default_inventory(), false, true));
    assert!(f.contains("\r\nM812\r\nT16M6 \r\nM811 S100\r\n") && f.contains("\r\nM812\r\nT13M6 \r\nM811 S100\r\n"));
    assert!(text(&o).contains("Spindle fan off during tool changes"));
    assert!(cli(&d, None, &[ENGRAVE, "-o", "N.cnc"]).status.success());
    assert!(!std::fs::read_to_string(d.p("N.cnc")).unwrap().contains("M812"));
}

#[test]
fn dry_run_writes_nothing() {
    let d = Dir::new("dry");
    let o = cli(&d, None, &[ENGRAVE, "-n"]);
    assert!(o.status.success(), "{}", text(&o));
    assert!(text(&o).contains("nothing saved"));
    assert!(!d.p("MERGED.cnc").exists());
}

#[test]
fn overwrites_only_when_allowed_and_never_the_input() {
    let d = Dir::new("protect");
    std::fs::write(d.p("MERGED.cnc"), "old").unwrap();
    let o = cli(&d, None, &[ENGRAVE]);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).contains("already exists"));
    assert_eq!(std::fs::read_to_string(d.p("MERGED.cnc")).unwrap(), "old");
    assert!(cli(&d, None, &[ENGRAVE, "-f"]).status.success());
    assert_ne!(std::fs::read_to_string(d.p("MERGED.cnc")).unwrap(), "old");
    let before = std::fs::read(d.p(ENGRAVE)).unwrap();
    let o = cli(&d, None, &[ENGRAVE, "-o", ENGRAVE, "-f"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).contains("one of the input files"));
    assert_eq!(std::fs::read(d.p(ENGRAVE)).unwrap(), before);
}

#[test]
fn manual_tool_numbers_and_no_inventory() {
    let d = Dir::new("tools");
    let o = cli(&d, None, &[ENGRAVE, CONTOUR, "--no-inventory", "-o", "A.cnc"]);
    assert!(o.status.success(), "{}", text(&o));
    let a = std::fs::read_to_string(d.p("A.cnc")).unwrap();
    assert!(a.contains("\r\nT1M6 \r\n") && a.contains("\r\nT2M6 \r\n"));

    let o = cli(&d, None, &[ENGRAVE, CONTOUR, "-t", "2=7", "--werkzeug", "1:1=8", "-o", "B.cnc"]);
    assert!(o.status.success(), "{}", text(&o));
    let b = std::fs::read_to_string(d.p("B.cnc")).unwrap();
    assert!(b.contains("\r\nT8M6 \r\n") && b.contains("\r\nT7M6 \r\n"));

    let o = cli(&d, None, &[ENGRAVE, CONTOUR, "-t", "1=13", "-o", "C.cnc"]);
    assert!(o.status.success());
    assert!(text(&o).contains("T13 stands for different tools"));

    let o = cli(&d, None, &[ENGRAVE, "-t", "3=5"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(text(&o).contains("there is no part 3"));
    let o = cli(&d, None, &[ENGRAVE, "-t", "1:4=5"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(text(&o).contains("does not use tool T4"));
}

#[test]
fn inventory_from_settings_or_file() {
    let d = Dir::new("inventory");
    let inv = r#"[{"description":"Single Flute Engraving Metal 30 deg*.3mm","number":21},
                  {"description":"Spiral O Metal 3.175*12mm Makera","number":22}]"#;
    std::fs::write(d.p("settings.json"), format!(r#"{{"lang":"en","safeZ":2.0,"inventory":{inv}}}"#)).unwrap();
    std::fs::write(d.p("inv.json"), inv.replace("21", "31").replace("22", "32")).unwrap();

    let o = cli(&d, Some(&d.p("settings.json")), &[ENGRAVE, CONTOUR, "--rapid", "-o", "S.cnc"]);
    assert!(o.status.success(), "{}", text(&o));
    let s = std::fs::read_to_string(d.p("S.cnc")).unwrap();
    assert!(s.contains("\r\nT21M6 \r\n") && s.contains("\r\nT22M6 \r\n"));
    assert!(text(&o).contains("Safe height: 2 mm"));

    let o = cli(&d, Some(&d.p("settings.json")), &[ENGRAVE, CONTOUR, "--inventory", "inv.json", "-o", "I.cnc"]);
    assert!(o.status.success(), "{}", text(&o));
    let i = std::fs::read_to_string(d.p("I.cnc")).unwrap();
    assert!(i.contains("\r\nT31M6 \r\n") && i.contains("\r\nT32M6 \r\n"));

    let o = cli(&d, None, &[ENGRAVE, "--inventory", ENGRAVE]);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).contains("no inventory found"));
}

#[test]
fn already_merged_files_are_skipped() {
    let d = Dir::new("merged");
    assert!(cli(&d, None, &[ENGRAVE, CONTOUR, "-o", "MERGED_old.cnc"]).status.success());
    let german =
        std::fs::read_to_string(d.p("MERGED_old.cnc")).unwrap().replacen("(Merged with ", "(Zusammengefuegt mit ", 1);
    std::fs::write(d.p("GESAMT_old.cnc"), german).unwrap();
    let o = cli(&d, None, &[ENGRAVE, "MERGED_old.cnc", "GESAMT_old.cnc", CONTOUR, "--stamp", STAMP]);
    assert!(o.status.success(), "{}", text(&o));
    assert!(text(&o).contains("MERGED_old.cnc skipped") && text(&o).contains("GESAMT_old.cnc skipped"));
    assert_eq!(
        std::fs::read_to_string(d.p("MERGED.cnc")).unwrap(),
        expected(&d, &[ENGRAVE, CONTOUR], &default_inventory(), false, false)
    );
    let o = cli(&d, None, &["MERGED_old.cnc", "-o", "X.cnc"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).contains("already merged by MultiPass"));
}

#[test]
fn errors_and_help() {
    let d = Dir::new("errors");
    let o = cli(&d, None, &["does_not_exist.cnc"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).starts_with("ERROR:"));
    assert_eq!(cli(&d, None, &[]).status.code(), Some(2));
    let h = cli(&d, None, &["--help"]);
    assert!(h.status.success() && text(&h).contains("--tool") && text(&h).contains("--fan-pause"));
    assert!(text(&cli(&d, None, &["--hilfe"])).contains("Usage:"));
    assert!(text(&cli(&d, None, &["-V"])).contains(env!("CARGO_PKG_VERSION")));
}
