use crate::error::NcError;
use crate::ncfile::{NcFile, TOOLCHANGE, TOOLCHANGE_SUB};
use crate::pycompat::{parse_uint, regex, strip};
use crate::rapid::{restore_rapids, RapidStats};
use crate::tools::ToolMap;
use regex::{Captures, Regex};
use serde::Serialize;
use std::borrow::{Borrow, Cow};
use std::fmt;
use std::sync::LazyLock;

pub const APP_NAME: &str = "MultiPass";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

static PROLOG_TOOL: LazyLock<Regex> = LazyLock::new(|| regex(r"^\(T(\d+)(\s.*)$"));

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "code", rename_all = "camelCase")]
pub enum Warning {
    ZOriginDiffers { origins: Vec<String> },
    OffsetsDiffer,
    MachinesDiffer { models: Vec<String> },
    RapidOffIncremental,
    RapidOffZOrigin { files: Vec<String> },
    RapidOffNonAscii { file: String, line: usize },
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Warning::ZOriginDiffers { origins } => {
                write!(f, "Unterschiedlicher Z-Nullpunkt in den Dateien: {}", origins.join(", "))
            }
            Warning::OffsetsDiffer => write!(
                f,
                "Unterschiedliche X/Y-Versätze in den Dateiköpfen – stammen die Dateien aus demselben Setup?"
            ),
            Warning::MachinesDiffer { models } => {
                write!(f, "Dateien für unterschiedliche Maschinen: {}", models.join(", "))
            }
            Warning::RapidOffIncremental => {
                write!(f, "Eine Datei nutzt G91 (inkrementell) – Eilgang-Umwandlung wurde deshalb abgeschaltet.")
            }
            Warning::RapidOffZOrigin { files } => write!(
                f,
                "Z-Nullpunkt ist nicht die Materialoberkante (Stock Top) in: {} – Eilgang-Umwandlung wurde deshalb abgeschaltet.",
                files.join(", ")
            ),
            Warning::RapidOffNonAscii { file, line } => write!(
                f,
                "{file}: ungewöhnliche Zeichen in Zeile {line} – Eilgang-Umwandlung für diesen Teil abgeschaltet."
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MergeOptions {
    pub restore: bool,
    pub safe_z: f64,
    pub stamp: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeResult {
    pub text: String,
    pub report: Vec<String>,
    pub warnings: Vec<Warning>,
    pub rapid: Option<RapidStats>,
    pub parts: Vec<Option<RapidStats>>,
}

pub fn ascii_safe(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            'ä' => out.push_str("ae"),
            'ö' => out.push_str("oe"),
            'ü' => out.push_str("ue"),
            'Ä' => out.push_str("Ae"),
            'Ö' => out.push_str("Oe"),
            'Ü' => out.push_str("Ue"),
            'ß' => out.push_str("ss"),
            '(' => out.push('['),
            ')' => out.push(']'),
            c if c.is_ascii() => out.push(c),
            _ => out.push('_'),
        }
    }
    out
}

fn new_number(tm: &ToolMap, n: u64) -> u64 {
    tm.get(&n).copied().unwrap_or(n)
}

fn uniq_sorted<I: IntoIterator<Item = String>>(it: I) -> Vec<String> {
    let mut v: Vec<String> = it.into_iter().collect();
    v.sort();
    v.dedup();
    v
}

pub fn merge<F: Borrow<NcFile>>(
    files: &[F],
    tool_maps: &[ToolMap],
    opts: &MergeOptions,
) -> Result<MergeResult, NcError> {
    let files: Vec<&NcFile> = files.iter().map(Borrow::borrow).collect();
    let first = *files.first().ok_or(NcError::NoFiles)?;
    let empty = ToolMap::new();
    let tm_of = |i: usize| tool_maps.get(i).unwrap_or(&empty);
    let mut warnings = Vec::new();

    if files.iter().any(|f| f.units != first.units) {
        return Err(NcError::MixedUnits);
    }
    let zor = uniq_sorted(files.iter().filter_map(|f| f.zorigin.clone()).filter(|z| !z.is_empty()));
    if zor.len() > 1 {
        warnings.push(Warning::ZOriginDiffers { origins: zor });
    }
    let mut offs: Vec<(f64, Option<f64>)> = Vec::new();
    for f in &files {
        if let Some(x) = f.xoff {
            if !offs.contains(&(x, f.yoff)) {
                offs.push((x, f.yoff));
            }
        }
    }
    if offs.len() > 1 {
        warnings.push(Warning::OffsetsDiffer);
    }
    let models = uniq_sorted(files.iter().filter_map(|f| f.model.clone()).filter(|m| !m.is_empty()));
    if models.len() > 1 {
        warnings.push(Warning::MachinesDiffer { models });
    }
    let mut restore = opts.restore;
    if restore && files.iter().any(|f| f.incremental) {
        warnings.push(Warning::RapidOffIncremental);
        restore = false;
    }
    if restore {
        let bad: Vec<String> = files.iter().filter(|f| !f.is_stock_top()).map(|f| f.name.clone()).collect();
        if !bad.is_empty() {
            warnings.push(Warning::RapidOffZOrigin { files: bad });
            restore = false;
        }
    }

    let nl = first.newline;
    let total_lines: usize = files.iter().map(|f| f.lines.len()).sum();
    let total_bytes: usize = files.iter().flat_map(|f| f.lines.iter()).map(|l| l.len() + nl.len()).sum();
    let mut text = String::with_capacity(total_bytes + 4096 + total_lines / 8);
    let push = |text: &mut String, l: &str| {
        text.push_str(l);
        text.push_str(nl);
    };

    push(&mut text, &format!("(Zusammengefuegt mit {} {} am {})", ascii_safe(APP_NAME), APP_VERSION, opts.stamp));
    push(&mut text, "(Reihenfolge:)");
    for (i, f) in files.iter().enumerate() {
        let tm = tm_of(i);
        let tl: Vec<String> =
            f.tool_numbers.iter().map(|&n| format!("T{} {}", new_number(tm, n), f.tool(n).unwrap_or(""))).collect();
        push(&mut text, &format!("(  {}. {} -> {})", i + 1, ascii_safe(&f.name), ascii_safe(&tl.join(", "))));
    }
    push(&mut text, "");

    for l in first.prolog() {
        if let Some(c) = PROLOG_TOOL.captures(strip(l)) {
            if parse_uint(&c[1]).is_some_and(|n| first.tool(n).is_some()) {
                continue;
            }
        }
        push(&mut text, l);
    }
    let mut seen: Vec<(u64, String)> = Vec::new();
    for (i, f) in files.iter().enumerate() {
        for &n in &f.tool_numbers {
            let t = new_number(tm_of(i), n);
            if !seen.iter().any(|(k, _)| *k == t) {
                seen.push((t, f.tool(n).unwrap_or("").to_string()));
            }
        }
    }
    seen.sort_by_key(|(t, _)| *t);
    push(&mut text, "(Werkzeuge:)");
    for (t, d) in &seen {
        push(&mut text, &format!("(T{t}  {})", ascii_safe(d)));
    }
    push(&mut text, "");

    let mut report = Vec::new();
    let mut parts = Vec::with_capacity(files.len());
    let mut total = RapidStats::default();
    for (i, f) in files.iter().enumerate() {
        let tm = tm_of(i);
        let body: Vec<Cow<str>> = f
            .body()
            .iter()
            .map(|l| {
                if TOOLCHANGE.is_match(l) {
                    TOOLCHANGE_SUB.replace_all(l, |c: &Captures| match parse_uint(&c[1]) {
                        Some(old) => format!("T{}{}", new_number(tm, old), &c[2]),
                        None => c[0].to_string(),
                    })
                } else {
                    Cow::Borrowed(l.as_str())
                }
            })
            .collect();
        push(&mut text, &format!("(===== Teil {}: {} =====)", i + 1, ascii_safe(&f.name)));
        let mut part = None;
        if restore {
            match restore_rapids(&body, opts.safe_z) {
                Ok((converted, stats)) => {
                    for l in &converted {
                        push(&mut text, l);
                    }
                    total.add(&stats);
                    part = Some(stats);
                }
                Err(e) => {
                    warnings.push(Warning::RapidOffNonAscii { file: f.name.clone(), line: f.body_start + e.line + 1 });
                    for l in &body {
                        push(&mut text, l);
                    }
                }
            }
        } else {
            for l in &body {
                push(&mut text, l);
            }
        }
        parts.push(part);
        push(&mut text, "");
        let tl: Vec<String> =
            f.tool_numbers.iter().map(|&n| format!("T{} ({})", new_number(tm, n), f.tool(n).unwrap_or(""))).collect();
        report.push(format!("{}. {}  ->  {}", i + 1, f.name, tl.join(", ")));
    }

    for l in files[files.len() - 1].epilog() {
        push(&mut text, l);
    }
    let rapid = restore.then_some(total);
    if let Some(t) = rapid {
        report.push(String::new());
        report.push(format!(
            "Eilgang: {} Leerfahrten umgewandelt ({:.1} m, vorher ca. {:.0} min im Vorschub).",
            t.converted,
            t.distance_mm / 1000.0,
            t.feed_time_min
        ));
    }
    Ok(MergeResult { text, report, warnings, rapid, parts })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::default_tool_map;

    #[test]
    fn ascii_umschreibung() {
        assert_eq!(ascii_safe("Fräser Ø3 (lang) Übung ß"), "Fraeser _3 [lang] Uebung ss");
    }

    fn nc(name: &str, head: &str, body: &str) -> NcFile {
        let src = format!("(T1  Fraeser  D=3.)\n{head}\nG90\n(op)\nT1M6\n{body}\nM30\n");
        NcFile::parse(name, src.as_bytes()).unwrap()
    }

    fn opts(restore: bool) -> MergeOptions {
        MergeOptions { restore, safe_z: 1.0, stamp: "S".into() }
    }

    const BODY: &str = "G0 X0 Y0 Z10\nG1 X5 F100";
    const TOP: &str = "(   Z Origin Set To   : Stock Top )";

    #[test]
    fn eilgang_gesperrt_wenn_nullpunkt_nicht_oben() {
        let files = [nc("a.nc", TOP, BODY), nc("b.nc", "(   Z Origin Set To   : Model Bottom )", BODY)];
        let r = merge(&files, &default_tool_map(&files), &opts(true)).unwrap();
        assert!(r.rapid.is_none());
        assert!(!r.text.contains("G0 X5."));
        assert!(r.warnings.contains(&Warning::RapidOffZOrigin { files: vec!["b.nc".into()] }));
        assert!(r.warnings.iter().any(|w| matches!(w, Warning::ZOriginDiffers { .. })));

        let files = [nc("a.nc", "", BODY)];
        let r = merge(&files, &default_tool_map(&files), &opts(true)).unwrap();
        assert!(r.rapid.is_none());
        assert!(r.warnings.contains(&Warning::RapidOffZOrigin { files: vec!["a.nc".into()] }));

        let files = [nc("a.nc", TOP, BODY)];
        let r = merge(&files, &default_tool_map(&files), &opts(true)).unwrap();
        assert_eq!(r.rapid.map(|s| s.converted), Some(1));
        assert!(r.warnings.is_empty());
    }

    #[test]
    fn eilgang_gesperrt_bei_g91() {
        let files = [nc("a.nc", TOP, "G91\nG1 X5 F100")];
        let r = merge(&files, &default_tool_map(&files), &opts(true)).unwrap();
        assert!(r.rapid.is_none());
        assert_eq!(r.warnings, vec![Warning::RapidOffIncremental]);
    }

    #[test]
    fn nicht_ascii_teil_bleibt_unveraendert() {
        let files = [nc("a.nc", TOP, "G0 X0 Y0 Z10\nG1 X5 F100\nG1 Y\u{0661}")];
        let r = merge(&files, &default_tool_map(&files), &opts(true)).unwrap();
        assert!(r.text.contains("G1 X5 F100"));
        assert_eq!(r.parts, vec![None]);
        assert!(matches!(r.warnings[0], Warning::RapidOffNonAscii { line: 8, .. }));
    }

    #[test]
    fn einheiten_fehler_und_warnungen() {
        let files = [nc("a.nc", "G20", BODY), nc("b.nc", "G21", BODY)];
        assert_eq!(merge(&files, &default_tool_map(&files), &opts(false)).unwrap_err(), NcError::MixedUnits);
        assert_eq!(merge::<NcFile>(&[], &[], &opts(false)).unwrap_err(), NcError::NoFiles);
        let files = [
            nc("a.nc", "(   X Offset   : 0. mm )\n(  model: Z1)", BODY),
            nc("b.nc", "(   X Offset   : 5. mm )\n(  model: C1)", BODY),
        ];
        let r = merge(&files, &default_tool_map(&files), &opts(false)).unwrap();
        assert_eq!(
            r.warnings,
            vec![Warning::OffsetsDiffer, Warning::MachinesDiffer { models: vec!["C1".into(), "Z1".into()] }]
        );
    }

    #[test]
    fn manuelle_nummern() {
        let files = [nc("a.nc", TOP, BODY), nc("b.nc", TOP, BODY)];
        let maps = vec![ToolMap::from([(1, 13)]), ToolMap::from([(1, 9)])];
        let r = merge(&files, &maps, &opts(false)).unwrap();
        let tcs: Vec<&str> = r.text.lines().filter(|l| TOOLCHANGE.is_match(l)).collect();
        assert_eq!(tcs, vec!["T13M6", "T9M6"]);
        assert!(r.text.contains("(Werkzeuge:)\n(T9  Fraeser)\n(T13  Fraeser)\n"));
        assert_eq!(r.text.matches("M30").count(), 1);
    }
}
