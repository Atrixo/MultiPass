use multipass_core::{assign_tools, default_inventory, merge, InventoryEntry, MergeOptions, NcFile};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const STAMP: &str = "2026-11-20 08:00";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Dir {
        let d = std::env::temp_dir().join(format!("multipass_cli_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        for f in ["4_T16_Gravur.cnc", "5_T13_Kontur.cnc"] {
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
        .env("MULTIPASS_SETTINGS", settings.map(Path::to_path_buf).unwrap_or_else(|| dir.p("keine-einstellungen.json")))
        .args(args)
        .output()
        .unwrap()
}

fn text(o: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
}

fn expected(dir: &Dir, names: &[&str], inventory: &[InventoryEntry], restore: bool) -> String {
    let files: Vec<NcFile> = names.iter().map(|n| NcFile::read(&dir.p(n)).unwrap()).collect();
    let maps = assign_tools(&files, inventory);
    merge(&files, &maps, &MergeOptions { restore, safe_z: 1.0, stamp: STAMP.into() }).unwrap().text
}

#[test]
fn gleiches_ergebnis_wie_die_oberflaeche() {
    let d = Dir::new("gleich");
    let o = cli(&d, None, &["5_T13_Kontur.cnc", "4_T16_Gravur.cnc", "--sortieren", "--stamp", STAMP]);
    assert!(o.status.success(), "{}", text(&o));
    let out = std::fs::read_to_string(d.p("GESAMT.cnc")).unwrap();
    assert_eq!(out, expected(&d, &["4_T16_Gravur.cnc", "5_T13_Kontur.cnc"], &default_inventory(), false));
    assert!(out.contains("\r\nT16M6 \r\n") && out.contains("\r\nT13M6 \r\n"));
    let t = text(&o);
    assert!(t.contains("Laufzeit geschaetzt") && t.contains("Werkzeugwechsel: 2") && t.contains("Gespeichert:"));
    assert!(!d.p("GESAMT.cnc.multipass-tmp").exists());
}

#[test]
fn eilgang_nur_auf_wunsch() {
    let d = Dir::new("eilgang");
    let o = cli(&d, None, &["4_T16_Gravur.cnc", "5_T13_Kontur.cnc", "--rapid", "--stamp", STAMP, "-o", "R.cnc"]);
    assert!(o.status.success(), "{}", text(&o));
    let names = ["4_T16_Gravur.cnc", "5_T13_Kontur.cnc"];
    assert_eq!(std::fs::read_to_string(d.p("R.cnc")).unwrap(), expected(&d, &names, &default_inventory(), true));
    assert!(text(&o).contains("Leerfahrten umgewandelt") && text(&o).contains("mit Eilgang"));
}

#[test]
fn nur_pruefen_schreibt_nichts() {
    let d = Dir::new("pruefen");
    let o = cli(&d, None, &["4_T16_Gravur.cnc", "-n"]);
    assert!(o.status.success(), "{}", text(&o));
    assert!(text(&o).contains("nichts gespeichert"));
    assert!(!d.p("GESAMT.cnc").exists());
}

#[test]
fn ueberschreibt_nur_mit_erlaubnis_und_nie_die_eingabe() {
    let d = Dir::new("schutz");
    std::fs::write(d.p("GESAMT.cnc"), "alt").unwrap();
    let o = cli(&d, None, &["4_T16_Gravur.cnc"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).contains("gibt es schon"));
    assert_eq!(std::fs::read_to_string(d.p("GESAMT.cnc")).unwrap(), "alt");
    assert!(cli(&d, None, &["4_T16_Gravur.cnc", "-f"]).status.success());
    assert_ne!(std::fs::read_to_string(d.p("GESAMT.cnc")).unwrap(), "alt");
    let before = std::fs::read(d.p("4_T16_Gravur.cnc")).unwrap();
    let o = cli(&d, None, &["4_T16_Gravur.cnc", "-o", "4_T16_Gravur.cnc", "-f"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).contains("Eingabedateien"));
    assert_eq!(std::fs::read(d.p("4_T16_Gravur.cnc")).unwrap(), before);
}

#[test]
fn werkzeugnummern_von_hand_und_ohne_inventar() {
    let d = Dir::new("werkzeug");
    let o = cli(&d, None, &["4_T16_Gravur.cnc", "5_T13_Kontur.cnc", "--ohne-inventar", "-o", "A.cnc"]);
    assert!(o.status.success(), "{}", text(&o));
    let a = std::fs::read_to_string(d.p("A.cnc")).unwrap();
    assert!(a.contains("\r\nT1M6 \r\n") && a.contains("\r\nT2M6 \r\n"));

    let o = cli(&d, None, &["4_T16_Gravur.cnc", "5_T13_Kontur.cnc", "-t", "2=7", "-t", "1:1=8", "-o", "B.cnc"]);
    assert!(o.status.success(), "{}", text(&o));
    let b = std::fs::read_to_string(d.p("B.cnc")).unwrap();
    assert!(b.contains("\r\nT8M6 \r\n") && b.contains("\r\nT7M6 \r\n"));

    let o = cli(&d, None, &["4_T16_Gravur.cnc", "5_T13_Kontur.cnc", "-t", "1=13", "-o", "C.cnc"]);
    assert!(o.status.success());
    assert!(text(&o).contains("T13 steht fuer verschiedene Werkzeuge"));

    let o = cli(&d, None, &["4_T16_Gravur.cnc", "-t", "3=5"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(text(&o).contains("Teil 3 gibt es nicht"));
    let o = cli(&d, None, &["4_T16_Gravur.cnc", "-t", "1:4=5"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(text(&o).contains("kein Werkzeug T4"));
}

#[test]
fn inventar_aus_einstellungen_oder_datei() {
    let d = Dir::new("inventar");
    let inv = r#"[{"description":"Single Flute Engraving Metal 30 deg*.3mm","number":21},
                  {"description":"Spiral O Metal 3.175*12mm Makera","number":22}]"#;
    std::fs::write(d.p("settings.json"), format!(r#"{{"lang":"de","safeZ":2.0,"inventory":{inv}}}"#)).unwrap();
    std::fs::write(d.p("inv.json"), inv.replace("21", "31").replace("22", "32")).unwrap();
    let files = ["4_T16_Gravur.cnc", "5_T13_Kontur.cnc"];

    let o = cli(&d, Some(&d.p("settings.json")), &[files[0], files[1], "--rapid", "-o", "S.cnc"]);
    assert!(o.status.success(), "{}", text(&o));
    let s = std::fs::read_to_string(d.p("S.cnc")).unwrap();
    assert!(s.contains("\r\nT21M6 \r\n") && s.contains("\r\nT22M6 \r\n"));
    assert!(text(&o).contains("Sicherheitshoehe: 2 mm"));

    let o = cli(&d, Some(&d.p("settings.json")), &[files[0], files[1], "--inventar", "inv.json", "-o", "I.cnc"]);
    assert!(o.status.success(), "{}", text(&o));
    let i = std::fs::read_to_string(d.p("I.cnc")).unwrap();
    assert!(i.contains("\r\nT31M6 \r\n") && i.contains("\r\nT32M6 \r\n"));

    let o = cli(&d, None, &[files[0], "--inventar", "4_T16_Gravur.cnc"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).contains("kein Inventar erkannt"));
}

#[test]
fn schon_zusammengefuegte_dateien_werden_uebersprungen() {
    let d = Dir::new("gesamt");
    assert!(cli(&d, None, &["4_T16_Gravur.cnc", "5_T13_Kontur.cnc", "-o", "GESAMT_alt.cnc"]).status.success());
    let o = cli(&d, None, &["4_T16_Gravur.cnc", "GESAMT_alt.cnc", "5_T13_Kontur.cnc", "--stamp", STAMP]);
    assert!(o.status.success(), "{}", text(&o));
    assert!(text(&o).contains("GESAMT_alt.cnc uebersprungen"));
    let names = ["4_T16_Gravur.cnc", "5_T13_Kontur.cnc"];
    assert_eq!(std::fs::read_to_string(d.p("GESAMT.cnc")).unwrap(), expected(&d, &names, &default_inventory(), false));
    let o = cli(&d, None, &["GESAMT_alt.cnc", "-o", "X.cnc"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).contains("schon mit MultiPass"));
}

#[test]
fn fehler_und_hilfe() {
    let d = Dir::new("fehler");
    let o = cli(&d, None, &["gibt_es_nicht.cnc"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(text(&o).starts_with("FEHLER:"));
    assert_eq!(cli(&d, None, &[]).status.code(), Some(2));
    let h = cli(&d, None, &["--hilfe"]);
    assert!(h.status.success() && text(&h).contains("--werkzeug"));
    assert!(text(&cli(&d, None, &["-V"])).contains(env!("CARGO_PKG_VERSION")));
}
