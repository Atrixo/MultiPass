use multipass_core::{
    assign_tools, default_inventory, estimate_file, merge, natural_key, rapid_allowed, write_atomic, Estimate,
    InventoryEntry, MergeOptions, NcFile, ToolMap, APP_VERSION, SAFE_Z_MAX, SAFE_Z_MIN,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const HILFE: &str = "MultiPass - NC-Programme mit Werkzeugwechseln zusammenfuegen

Aufruf: multipass-cli [OPTIONEN] DATEI...

Die Dateien werden in der angegebenen Reihenfolge zusammengefuegt.

  -o, --ausgabe DATEI       Zieldatei (Standard: GESAMT.<Endung> im Ordner der ersten Datei)
  -f, --ueberschreiben      vorhandene Zieldatei ersetzen
  -s, --sortieren           Dateien nach Namen sortieren wie in der Oberflaeche (2 vor 10)
  -n, --nur-pruefen         nichts schreiben, nur Ablauf, Hinweise und Laufzeit anzeigen

  -t, --werkzeug N=T        Teil N bekommt die Werkzeugnummer T, z. B. -t 2=16
  -t, --werkzeug N:A=T      in Teil N wird Werkzeug A zu T (Dateien mit mehreren Werkzeugen)
      --inventar DATEI      Inventar aus einer JSON-Datei (Liste oder settings.json)
      --ohne-inventar       Werkzeuge nur durchnummerieren (1, 2, 3 ...)

      --rapid               Leerfahrten ueber dem Material wieder als Eilgang (G0)
      --safe-z MM           Sicherheitshoehe fuer --rapid, 0.5 bis 20 mm
      --eilgang-tempo MM    Eilgang der Maschine in mm/min, nur fuer die Laufzeitschaetzung
      --stamp TEXT          fester Zeitstempel statt der aktuellen Zeit

  -h, --hilfe               diese Hilfe
  -V, --version             Versionsnummer

Ohne --inventar, --safe-z und --eilgang-tempo gelten die Einstellungen der Oberflaeche,
sonst die Standardwerte (Makera-Inventar, 1.0 mm, 3000 mm/min).
Die Eilgang-Umwandlung ist nur mit --rapid aktiv.

Rueckgabewert: 0 = fertig, 1 = Fehler, 2 = falscher Aufruf.";

const RAPID_SPEED_MIN: f64 = 1000.0;
const RAPID_SPEED_MAX: f64 = 15000.0;
const RAPID_SPEED_DEFAULT: f64 = 3000.0;
const TOOL_MAX: u64 = 99;

#[derive(Debug, Default, PartialEq)]
struct Config {
    inputs: Vec<PathBuf>,
    output: Option<PathBuf>,
    overwrite: bool,
    sort: bool,
    dry_run: bool,
    tools: Vec<ToolOverride>,
    inventory: Option<PathBuf>,
    no_inventory: bool,
    restore: bool,
    safe_z: Option<f64>,
    rapid_speed: Option<f64>,
    stamp: Option<String>,
    help: bool,
    version: bool,
}

#[derive(Debug, PartialEq)]
struct ToolOverride {
    part: usize,
    old: Option<u64>,
    new: u64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    inventory: Option<Vec<InventoryEntry>>,
    safe_z: Option<f64>,
    rapid_speed: Option<f64>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum InventoryFile {
    List(Vec<InventoryEntry>),
    Settings { inventory: Vec<InventoryEntry> },
}

enum Failure {
    Usage(String),
    Run(String),
}

fn usage(msg: impl Into<String>) -> Failure {
    Failure::Usage(msg.into())
}

fn number(v: &str, what: &str) -> Result<f64, Failure> {
    v.trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|x| x.is_finite())
        .ok_or_else(|| usage(format!("{what}: '{v}' ist keine Zahl")))
}

fn whole(v: &str, what: &str) -> Result<u64, Failure> {
    v.trim().parse::<u64>().map_err(|_| usage(format!("{what}: '{v}' ist keine ganze Zahl")))
}

fn parse_tool(v: &str) -> Result<ToolOverride, Failure> {
    let bad = || usage(format!("--werkzeug '{v}': erwartet N=T oder N:A=T, z. B. 2=16"));
    let (left, new) = v.split_once('=').ok_or_else(bad)?;
    let (part, old) = match left.split_once(':') {
        Some((p, a)) => (p, Some(whole(a, "--werkzeug")?)),
        None => (left, None),
    };
    let part = whole(part, "--werkzeug")? as usize;
    let new = whole(new, "--werkzeug")?;
    if part == 0 {
        return Err(usage("--werkzeug: Teile werden ab 1 gezaehlt"));
    }
    if !(1..=TOOL_MAX).contains(&new) {
        return Err(usage(format!("--werkzeug: Werkzeugnummer muss zwischen 1 und {TOOL_MAX} liegen")));
    }
    Ok(ToolOverride { part, old, new })
}

fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Config, Failure> {
    let mut c = Config::default();
    let mut it = args.into_iter();
    let mut only_files = false;
    while let Some(a) = it.next() {
        if only_files || !a.starts_with('-') || a == "-" {
            c.inputs.push(a.into());
            continue;
        }
        let (key, inline) = match a.split_once('=') {
            Some((k, v)) if k.starts_with("--") => (k.to_string(), Some(v.to_string())),
            _ => (a.clone(), None),
        };
        let mut value = |name: &str| -> Result<String, Failure> {
            match inline.clone() {
                Some(v) => Ok(v),
                None => it.next().ok_or_else(|| usage(format!("{name} braucht einen Wert"))),
            }
        };
        match key.as_str() {
            "--" => only_files = true,
            "-h" | "--hilfe" | "--help" => c.help = true,
            "-V" | "--version" => c.version = true,
            "-o" | "--ausgabe" | "--output" => c.output = Some(value(&key)?.into()),
            "-f" | "--ueberschreiben" | "--force" => c.overwrite = true,
            "-s" | "--sortieren" | "--sort" => c.sort = true,
            "-n" | "--nur-pruefen" | "--dry-run" => c.dry_run = true,
            "-t" | "--werkzeug" | "--tool" => c.tools.push(parse_tool(&value(&key)?)?),
            "--inventar" | "--inventory" => c.inventory = Some(value(&key)?.into()),
            "--ohne-inventar" | "--no-inventory" => c.no_inventory = true,
            "--rapid" => c.restore = true,
            "--safe-z" => {
                let z = number(&value(&key)?, "--safe-z")?;
                if !(SAFE_Z_MIN..=SAFE_Z_MAX).contains(&z) {
                    return Err(usage(format!("--safe-z muss zwischen {SAFE_Z_MIN} und {SAFE_Z_MAX} mm liegen")));
                }
                c.safe_z = Some(z);
            }
            "--eilgang-tempo" | "--rapid-speed" => {
                let s = number(&value(&key)?, "--eilgang-tempo")?;
                if !(RAPID_SPEED_MIN..=RAPID_SPEED_MAX).contains(&s) {
                    return Err(usage(format!(
                        "--eilgang-tempo muss zwischen {RAPID_SPEED_MIN} und {RAPID_SPEED_MAX} mm/min liegen"
                    )));
                }
                c.rapid_speed = Some(s);
            }
            "--stamp" => c.stamp = Some(value(&key)?),
            _ => return Err(usage(format!("unbekannte Option '{a}'"))),
        }
    }
    if c.inventory.is_some() && c.no_inventory {
        return Err(usage("--inventar und --ohne-inventar schliessen sich aus"));
    }
    Ok(c)
}

fn settings_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("MULTIPASS_SETTINGS") {
        return Some(p.into());
    }
    let id = "de.multipass.desktop";
    let base: PathBuf = if cfg!(windows) {
        std::env::var_os("APPDATA")?.into()
    } else if cfg!(target_os = "macos") {
        PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support")
    } else {
        match std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
            Some(d) => d.into(),
            None => PathBuf::from(std::env::var_os("HOME")?).join(".local/share"),
        }
    };
    Some(base.join(id).join("settings.json"))
}

fn load_settings() -> Settings {
    settings_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn load_inventory(path: &Path) -> Result<Vec<InventoryEntry>, Failure> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| Failure::Run(format!("{}: Inventar konnte nicht gelesen werden ({e})", path.display())))?;
    match serde_json::from_str::<InventoryFile>(&text) {
        Ok(InventoryFile::List(v)) | Ok(InventoryFile::Settings { inventory: v }) => Ok(v),
        Err(_) => Err(Failure::Run(format!(
            "{}: kein Inventar erkannt. Erwartet [{{\"description\": \"...\", \"number\": 13}}, ...]",
            path.display()
        ))),
    }
}

fn duration(min: f64) -> String {
    if !min.is_finite() || min < 0.0 {
        return "-".into();
    }
    if min < 1.0 {
        return "< 1 min".into();
    }
    let total = min.round() as u64;
    match (total / 60, total % 60) {
        (0, m) => format!("{m} min"),
        (h, m) => format!("{h} h {m:02} min"),
    }
}

fn apply_overrides(files: &[NcFile], maps: &mut [ToolMap], tools: &[ToolOverride]) -> Result<(), Failure> {
    for o in tools {
        let f = files.get(o.part - 1).ok_or_else(|| {
            usage(format!("--werkzeug: Teil {} gibt es nicht, es sind {} Dateien", o.part, files.len()))
        })?;
        let old = match o.old {
            Some(a) if f.tool_numbers.contains(&a) => a,
            Some(a) => return Err(usage(format!("--werkzeug: {} benutzt kein Werkzeug T{a}", f.name))),
            None => match f.tool_numbers.as_slice() {
                [a] => *a,
                _ => {
                    return Err(usage(format!(
                        "--werkzeug: {} hat mehrere Werkzeuge, bitte N:A=T angeben (A = T{})",
                        f.name,
                        f.tool_numbers.iter().map(u64::to_string).collect::<Vec<_>>().join(", T")
                    )))
                }
            },
        };
        maps[o.part - 1].insert(old, o.new);
    }
    Ok(())
}

fn number_conflicts(files: &[NcFile], maps: &[ToolMap]) -> Vec<String> {
    let mut by_number: BTreeMap<u64, Vec<String>> = BTreeMap::new();
    for (f, m) in files.iter().zip(maps) {
        for &n in &f.tool_numbers {
            let d = f.tool(n).unwrap_or("").to_string();
            let list = by_number.entry(m.get(&n).copied().unwrap_or(n)).or_default();
            if !list.contains(&d) {
                list.push(d);
            }
        }
    }
    by_number
        .into_iter()
        .filter(|(_, d)| d.len() > 1)
        .map(|(n, d)| format!("T{n} steht fuer verschiedene Werkzeuge: {}", d.join(" / ")))
        .collect()
}

fn run(c: Config) -> Result<(), Failure> {
    if c.help {
        println!("{HILFE}");
        return Ok(());
    }
    if c.version {
        println!("multipass-cli {APP_VERSION}");
        return Ok(());
    }
    if c.inputs.is_empty() {
        return Err(usage("keine Dateien angegeben (Hilfe: multipass-cli --hilfe)"));
    }
    let settings = load_settings();
    let safe_z = c.safe_z.or(settings.safe_z).filter(|z| (SAFE_Z_MIN..=SAFE_Z_MAX).contains(z)).unwrap_or(1.0);
    let rapid_speed = c
        .rapid_speed
        .or(settings.rapid_speed)
        .filter(|s| (RAPID_SPEED_MIN..=RAPID_SPEED_MAX).contains(s))
        .unwrap_or(RAPID_SPEED_DEFAULT);
    let inventory = match (&c.inventory, c.no_inventory) {
        (Some(p), _) => load_inventory(p)?,
        (None, true) => Vec::new(),
        (None, false) => settings.inventory.unwrap_or_else(default_inventory),
    };

    let mut inputs = c.inputs.clone();
    if c.sort {
        inputs
            .sort_by_key(|p| natural_key(&p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
    }
    let mut files = Vec::with_capacity(inputs.len());
    let mut skipped = Vec::new();
    let mut kept = Vec::with_capacity(inputs.len());
    for p in &inputs {
        let f = NcFile::read(p).map_err(|e| Failure::Run(e.to_string()))?;
        if f.is_merged() {
            skipped.push(f.name);
        } else {
            files.push(f);
            kept.push(p.clone());
        }
    }
    if files.is_empty() {
        return Err(Failure::Run("alle Dateien sind schon mit MultiPass zusammengefuegt".into()));
    }
    let inputs = kept;
    let mut maps = assign_tools(&files, &inventory);
    apply_overrides(&files, &mut maps, &c.tools)?;

    let output = match &c.output {
        Some(p) => p.clone(),
        None => {
            let first = &inputs[0];
            let ext = first.extension().map(|e| e.to_string_lossy().into_owned()).filter(|e| !e.is_empty());
            first.with_file_name(format!("GESAMT.{}", ext.as_deref().unwrap_or("cnc")))
        }
    };
    if !c.dry_run {
        let target = std::fs::canonicalize(&output).ok();
        if target.is_some() && inputs.iter().any(|p| std::fs::canonicalize(p).ok() == target) {
            return Err(Failure::Run(format!("{}: das ist eine der Eingabedateien", output.display())));
        }
        if output.exists() && !c.overwrite {
            return Err(Failure::Run(format!(
                "{} gibt es schon. Mit -f / --ueberschreiben ersetzen.",
                output.display()
            )));
        }
    }

    let stamp = c.stamp.clone().unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d %H:%M").to_string());
    let result = merge(&files, &maps, &MergeOptions { restore: c.restore, safe_z, stamp })
        .map_err(|e| Failure::Run(e.to_string()))?;

    let fast = c.restore && result.rapid.is_some() && rapid_allowed(&files);
    let estimates: Vec<(Estimate, Option<Estimate>)> = std::thread::scope(|s| {
        let handles: Vec<_> = files
            .iter()
            .map(|f| s.spawn(move || (estimate_file(f, None), fast.then(|| estimate_file(f, Some(safe_z))))))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or_default()).collect()
    });

    println!("MultiPass {APP_VERSION}");
    println!();
    for name in &skipped {
        println!("HINWEIS: {name} uebersprungen, ist schon mit MultiPass zusammengefuegt.");
    }
    if !skipped.is_empty() {
        println!();
    }
    for line in &result.report {
        println!("{line}");
    }
    println!();
    let mut plain_total = 0.0;
    let mut fast_total = 0.0;
    for (f, (p, q)) in files.iter().zip(&estimates) {
        let a = p.total_min(rapid_speed);
        let b = q.map(|q| q.total_min(rapid_speed)).unwrap_or(a);
        plain_total += a;
        fast_total += b;
        let mut line = format!("  {}: {}", f.name, duration(a));
        if q.is_some() {
            line.push_str(&format!(", mit Eilgang {}", duration(b)));
        }
        println!("{line}");
    }
    let mut total = format!("Laufzeit geschaetzt (Eilgang {rapid_speed:.0} mm/min): {}", duration(plain_total));
    if fast {
        total.push_str(&format!(", mit Eilgang {}", duration(fast_total)));
    }
    println!("{total}");
    let changes = files.iter().map(|f| f.tool_numbers.len()).sum::<usize>();
    println!("Werkzeugwechsel: {changes}");
    if c.restore {
        println!("Sicherheitshoehe: {safe_z} mm");
    }
    for w in &result.warnings {
        println!("HINWEIS: {w}");
    }
    for w in number_conflicts(&files, &maps) {
        println!("HINWEIS: {w}");
    }
    if c.dry_run {
        println!("Nur geprueft, nichts gespeichert.");
        return Ok(());
    }
    write_atomic(&output, result.text.as_bytes())
        .map_err(|e| Failure::Run(format!("{}: konnte nicht gespeichert werden ({e})", output.display())))?;
    println!("Gespeichert: {} ({} Zeilen)", output.display(), result.text.lines().count());
    Ok(())
}

fn main() -> ExitCode {
    match parse_args(std::env::args().skip(1)).and_then(run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Run(e)) => {
            eprintln!("FEHLER: {e}");
            ExitCode::from(1)
        }
        Err(Failure::Usage(e)) => {
            eprintln!("FEHLER: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    fn ok(s: &str) -> Config {
        match parse_args(args(s)) {
            Ok(c) => c,
            Err(Failure::Usage(e) | Failure::Run(e)) => panic!("{e}"),
        }
    }

    fn err(s: &str) -> String {
        match parse_args(args(s)) {
            Ok(_) => panic!("kein Fehler bei '{s}'"),
            Err(Failure::Usage(e) | Failure::Run(e)) => e,
        }
    }

    #[test]
    fn optionen_deutsch_und_englisch() {
        let c = ok("a.cnc --rapid --safe-z=2,5 -t 2=16 --tool 1:3=9 -o x.cnc -f -s -n b.cnc");
        assert_eq!(c.inputs, vec![PathBuf::from("a.cnc"), PathBuf::from("b.cnc")]);
        assert_eq!(c.safe_z, Some(2.5));
        assert_eq!(c.output, Some(PathBuf::from("x.cnc")));
        assert!(c.restore && c.overwrite && c.sort && c.dry_run);
        assert_eq!(
            c.tools,
            vec![ToolOverride { part: 2, old: None, new: 16 }, ToolOverride { part: 1, old: Some(3), new: 9 }]
        );
        assert_eq!(ok("-- -komisch.cnc").inputs, vec![PathBuf::from("-komisch.cnc")]);
    }

    #[test]
    fn falsche_werte_werden_abgelehnt() {
        assert!(err("--safe-z 0.1").contains("zwischen"));
        assert!(err("--safe-z abc").contains("keine Zahl"));
        assert!(err("-t 0=5").contains("ab 1"));
        assert!(err("-t 1=100").contains("zwischen 1 und 99"));
        assert!(err("-t 16").contains("N=T"));
        assert!(err("--eilgang-tempo 50").contains("zwischen"));
        assert!(err("--gibtsnicht").contains("unbekannte Option"));
        assert!(err("-o").contains("braucht einen Wert"));
        assert!(err("--inventar x.json --ohne-inventar").contains("schliessen sich aus"));
    }

    #[test]
    fn dauer_wie_in_der_oberflaeche() {
        assert_eq!(duration(0.4), "< 1 min");
        assert_eq!(duration(59.6), "1 h 00 min");
        assert_eq!(duration(125.0), "2 h 05 min");
        assert_eq!(duration(f64::NAN), "-");
    }
}
