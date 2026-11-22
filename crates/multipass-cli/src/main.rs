use multipass_core::{
    assign_tools, default_inventory, estimate_file, merge, natural_key, rapid_allowed, write_atomic, Estimate,
    InventoryEntry, MergeOptions, NcFile, ToolMap, APP_VERSION, SAFE_Z_MAX, SAFE_Z_MIN,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const HELP: &str = "MultiPass - merge NC programs into one program with tool changes

Usage: multipass-cli [OPTIONS] FILE...

The files are merged in the order given.

  -o, --output FILE         output file (default: MERGED.<ext> in the folder of the first file)
  -f, --force               replace an existing output file
  -s, --sort                sort files by name like the GUI does (2 before 10)
  -n, --dry-run             write nothing, only show order, notes and run time

  -t, --tool N=T            part N gets tool number T, e.g. -t 2=16
  -t, --tool N:A=T          in part N, tool A becomes T (files with several tools)
      --inventory FILE      tool inventory from a JSON file (list or settings.json)
      --no-inventory        just number the tools (1, 2, 3 ...)

      --rapid               turn air moves above the stock back into rapids (G0)
      --safe-z MM           safe height for --rapid, 0.5 to 20 mm
      --rapid-speed MM      machine rapid speed in mm/min, only for the run time estimate
      --fan-pause           spindle fan off during tool changes (M812 before, M811 S100 after)
      --stamp TEXT          fixed time stamp instead of the current time

  -h, --help                this help
  -V, --version             version number

Without --inventory, --safe-z and --rapid-speed the GUI settings are used,
otherwise the defaults (Makera inventory, 1.0 mm, 3000 mm/min).
Rapid conversion is only active with --rapid.
German option names (--ausgabe, --werkzeug, ...) still work.

Exit code: 0 = done, 1 = error, 2 = wrong usage.";

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
    fan_pause: bool,
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
        .ok_or_else(|| usage(format!("{what}: '{v}' is not a number")))
}

fn whole(v: &str, what: &str) -> Result<u64, Failure> {
    v.trim().parse::<u64>().map_err(|_| usage(format!("{what}: '{v}' is not a whole number")))
}

fn parse_tool(v: &str) -> Result<ToolOverride, Failure> {
    let bad = || usage(format!("--tool '{v}': expected N=T or N:A=T, e.g. 2=16"));
    let (left, new) = v.split_once('=').ok_or_else(bad)?;
    let (part, old) = match left.split_once(':') {
        Some((p, a)) => (p, Some(whole(a, "--tool")?)),
        None => (left, None),
    };
    let part = whole(part, "--tool")? as usize;
    let new = whole(new, "--tool")?;
    if part == 0 {
        return Err(usage("--tool: parts are counted from 1"));
    }
    if !(1..=TOOL_MAX).contains(&new) {
        return Err(usage(format!("--tool: tool number must be between 1 and {TOOL_MAX}")));
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
                None => it.next().ok_or_else(|| usage(format!("{name} needs a value"))),
            }
        };
        match key.as_str() {
            "--" => only_files = true,
            "-h" | "--help" | "--hilfe" => c.help = true,
            "-V" | "--version" => c.version = true,
            "-o" | "--output" | "--ausgabe" => c.output = Some(value(&key)?.into()),
            "-f" | "--force" | "--ueberschreiben" => c.overwrite = true,
            "-s" | "--sort" | "--sortieren" => c.sort = true,
            "-n" | "--dry-run" | "--nur-pruefen" => c.dry_run = true,
            "-t" | "--tool" | "--werkzeug" => c.tools.push(parse_tool(&value(&key)?)?),
            "--inventory" | "--inventar" => c.inventory = Some(value(&key)?.into()),
            "--no-inventory" | "--ohne-inventar" => c.no_inventory = true,
            "--rapid" => c.restore = true,
            "--safe-z" | "--sicherheitshoehe" => {
                let z = number(&value(&key)?, "--safe-z")?;
                if !(SAFE_Z_MIN..=SAFE_Z_MAX).contains(&z) {
                    return Err(usage(format!("--safe-z must be between {SAFE_Z_MIN} and {SAFE_Z_MAX} mm")));
                }
                c.safe_z = Some(z);
            }
            "--rapid-speed" | "--eilgang-tempo" => {
                let s = number(&value(&key)?, "--rapid-speed")?;
                if !(RAPID_SPEED_MIN..=RAPID_SPEED_MAX).contains(&s) {
                    return Err(usage(format!(
                        "--rapid-speed must be between {RAPID_SPEED_MIN} and {RAPID_SPEED_MAX} mm/min"
                    )));
                }
                c.rapid_speed = Some(s);
            }
            "--stamp" => c.stamp = Some(value(&key)?),
            "--fan-pause" | "--luefter-pause" => c.fan_pause = true,
            _ => return Err(usage(format!("unknown option '{a}'"))),
        }
    }
    if c.inventory.is_some() && c.no_inventory {
        return Err(usage("--inventory and --no-inventory cannot be combined"));
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
        .map_err(|e| Failure::Run(format!("{}: could not read the inventory ({e})", path.display())))?;
    match serde_json::from_str::<InventoryFile>(&text) {
        Ok(InventoryFile::List(v)) | Ok(InventoryFile::Settings { inventory: v }) => Ok(v),
        Err(_) => Err(Failure::Run(format!(
            "{}: no inventory found. Expected [{{\"description\": \"...\", \"number\": 13}}, ...]",
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
        let f = files
            .get(o.part - 1)
            .ok_or_else(|| usage(format!("--tool: there is no part {}, there are {} files", o.part, files.len())))?;
        let old = match o.old {
            Some(a) if f.tool_numbers.contains(&a) => a,
            Some(a) => return Err(usage(format!("--tool: {} does not use tool T{a}", f.name))),
            None => match f.tool_numbers.as_slice() {
                [a] => *a,
                _ => {
                    return Err(usage(format!(
                        "--tool: {} has several tools, please use N:A=T (A = T{})",
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
        .map(|(n, d)| format!("T{n} stands for different tools: {}", d.join(" / ")))
        .collect()
}

fn run(c: Config) -> Result<(), Failure> {
    if c.help {
        println!("{HELP}");
        return Ok(());
    }
    if c.version {
        println!("multipass-cli {APP_VERSION}");
        return Ok(());
    }
    if c.inputs.is_empty() {
        return Err(usage("no files given (help: multipass-cli --help)"));
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
        return Err(Failure::Run("all files are already merged by MultiPass".into()));
    }
    let inputs = kept;
    let mut maps = assign_tools(&files, &inventory);
    apply_overrides(&files, &mut maps, &c.tools)?;

    let output = match &c.output {
        Some(p) => p.clone(),
        None => {
            let first = &inputs[0];
            let ext = first.extension().map(|e| e.to_string_lossy().into_owned()).filter(|e| !e.is_empty());
            first.with_file_name(format!("MERGED.{}", ext.as_deref().unwrap_or("cnc")))
        }
    };
    if !c.dry_run {
        let target = std::fs::canonicalize(&output).ok();
        if target.is_some() && inputs.iter().any(|p| std::fs::canonicalize(p).ok() == target) {
            return Err(Failure::Run(format!("{}: this is one of the input files", output.display())));
        }
        if output.exists() && !c.overwrite {
            return Err(Failure::Run(format!("{} already exists. Use -f / --force to replace it.", output.display())));
        }
    }

    let stamp = c.stamp.clone().unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d %H:%M").to_string());
    let result = merge(&files, &maps, &MergeOptions { restore: c.restore, safe_z, stamp, fan_pause: c.fan_pause })
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
        println!("NOTE: {name} skipped, it is already merged by MultiPass.");
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
            line.push_str(&format!(", with rapids {}", duration(b)));
        }
        println!("{line}");
    }
    let mut total = format!("Estimated run time (rapid {rapid_speed:.0} mm/min): {}", duration(plain_total));
    if fast {
        total.push_str(&format!(", with rapids {}", duration(fast_total)));
    }
    println!("{total}");
    let order: Vec<u64> = files
        .iter()
        .zip(&maps)
        .flat_map(|(f, m)| f.tool_numbers.iter().map(move |n| m.get(n).copied().unwrap_or(*n)))
        .collect();
    let changes = order.windows(2).filter(|w| w[0] != w[1]).count();
    println!(
        "Tool changes: {changes} (plus inserting the first tool at the start; the machine asks {} times)",
        order.len()
    );
    if c.restore {
        println!("Safe height: {safe_z} mm");
    }
    if c.fan_pause {
        println!("Spindle fan off during tool changes (M812 / M811 S100).");
    }
    for w in &result.warnings {
        println!("NOTE: {w}");
    }
    for w in number_conflicts(&files, &maps) {
        println!("NOTE: {w}");
    }
    if c.dry_run {
        println!("Dry run, nothing saved.");
        return Ok(());
    }
    write_atomic(&output, result.text.as_bytes())
        .map_err(|e| Failure::Run(format!("{}: could not be saved ({e})", output.display())))?;
    println!("Saved: {} ({} lines)", output.display(), result.text.lines().count());
    Ok(())
}

fn main() -> ExitCode {
    match parse_args(std::env::args().skip(1)).and_then(run) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Run(e)) => {
            eprintln!("ERROR: {e}");
            ExitCode::from(1)
        }
        Err(Failure::Usage(e)) => {
            eprintln!("ERROR: {e}");
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
            Ok(_) => panic!("no error for '{s}'"),
            Err(Failure::Usage(e) | Failure::Run(e)) => e,
        }
    }

    #[test]
    fn options_german_and_english() {
        let c = ok("a.cnc --rapid --safe-z=2,5 -t 2=16 --werkzeug 1:3=9 --ausgabe x.cnc -f -s -n --fan-pause b.cnc");
        assert_eq!(c.inputs, vec![PathBuf::from("a.cnc"), PathBuf::from("b.cnc")]);
        assert_eq!(c.safe_z, Some(2.5));
        assert_eq!(c.output, Some(PathBuf::from("x.cnc")));
        assert!(c.restore && c.overwrite && c.sort && c.dry_run && c.fan_pause);
        assert_eq!(
            c.tools,
            vec![ToolOverride { part: 2, old: None, new: 16 }, ToolOverride { part: 1, old: Some(3), new: 9 }]
        );
        assert_eq!(ok("-- -odd.cnc").inputs, vec![PathBuf::from("-odd.cnc")]);
    }

    #[test]
    fn invalid_values_are_rejected() {
        assert!(err("--safe-z 0.1").contains("between"));
        assert!(err("--safe-z abc").contains("not a number"));
        assert!(err("-t 0=5").contains("from 1"));
        assert!(err("-t 1=100").contains("between 1 and 99"));
        assert!(err("-t 16").contains("N=T"));
        assert!(err("--rapid-speed 50").contains("between"));
        assert!(err("--nonexistent").contains("unknown option"));
        assert!(err("-o").contains("needs a value"));
        assert!(err("--inventory x.json --no-inventory").contains("cannot be combined"));
    }

    #[test]
    fn duration_like_in_the_gui() {
        assert_eq!(duration(0.4), "< 1 min");
        assert_eq!(duration(59.6), "1 h 00 min");
        assert_eq!(duration(125.0), "2 h 05 min");
        assert_eq!(duration(f64::NAN), "-");
    }
}
