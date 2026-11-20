use crate::error::NcError;
use crate::pycompat::{parse_float, parse_uint, regex, splitlines, strip};
use regex::Regex;
use serde::Serialize;
use std::path::Path;
use std::sync::LazyLock;

pub(crate) static TOOLCHANGE: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)^\s*T\s*(\d+)\s*M0?6\b"));
pub(crate) static TOOLCHANGE_SUB: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)\bT\s*(\d+)(\s*M0?6\b)"));
pub(crate) static END: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)^\s*M0?(30|2)\b"));
pub(crate) static COMMENT: LazyLock<Regex> = LazyLock::new(|| regex(r"\([^)]*\)|;.*$"));
static HEADER_TOOL: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)^\(T(\d+)\s+(.*?)\s{2,}D="));
static HEADER_TOOL_FALLBACK: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)^\(T(\d+)\s+(.*?)\)\s*$"));
static ZMIN: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)Z\s*Min\s+from\s+Stock\s+Top\s*:\s*(-?[\d.]+)"));
static ZORIGIN: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)Z\s*Origin\s+Set\s+To\s*:\s*(.*?)\s*\)"));
static XOFF: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)^\(\s*X\s+Offset\s*:\s*(-?[\d.]+)"));
static YOFF: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)^\(\s*Y\s+Offset\s*:\s*(-?[\d.]+)"));
static MODEL: LazyLock<Regex> = LazyLock::new(|| regex(r"(?i)model\s*:\s*(.*?)\s*\)"));
static WHITESPACE: LazyLock<Regex> = LazyLock::new(|| regex(r"\s+"));
static G20: LazyLock<Regex> = LazyLock::new(|| regex(r"\bG20\b"));
static G91: LazyLock<Regex> = LazyLock::new(|| regex(r"\bG91\b"));

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Units {
    Mm,
    Inch,
}

#[derive(Debug, Clone)]
pub struct NcFile {
    pub name: String,
    pub newline: &'static str,
    pub lines: Vec<String>,
    pub body_start: usize,
    pub body_end: usize,
    pub tools: Vec<(u64, String)>,
    pub tool_numbers: Vec<u64>,
    pub zmin: Option<f64>,
    pub zorigin: Option<String>,
    pub xoff: Option<f64>,
    pub yoff: Option<f64>,
    pub model: Option<String>,
    pub units: Units,
    pub incremental: bool,
}

impl NcFile {
    pub fn read(path: &Path) -> Result<NcFile, NcError> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        let bytes = std::fs::read(path).map_err(|e| NcError::Io { file: name.clone(), message: e.to_string() })?;
        NcFile::parse(&name, &bytes)
    }

    pub fn parse(name: &str, bytes: &[u8]) -> Result<NcFile, NcError> {
        let raw = String::from_utf8_lossy(bytes);
        let newline = if raw.contains("\r\n") { "\r\n" } else { "\n" };
        let lines: Vec<String> = splitlines(&raw).into_iter().map(str::to_owned).collect();
        let file = || name.to_string();
        let bad = |i: usize| NcError::BadNumber { file: name.to_string(), line: i + 1 };

        let first_tc =
            lines.iter().position(|l| TOOLCHANGE.is_match(l)).ok_or_else(|| NcError::NoToolChange { file: file() })?;
        let mut body_start = first_tc;
        if body_start > 0 && strip(&lines[body_start - 1]).starts_with('(') {
            body_start -= 1;
        }
        let body_end =
            lines.iter().rposition(|l| END.is_match(l)).ok_or_else(|| NcError::NoProgramEnd { file: file() })?;
        if body_end <= body_start {
            return Err(NcError::Structure { file: file() });
        }

        let mut tools: Vec<(u64, String)> = Vec::new();
        let mut zmin = None;
        let mut zorigin = None;
        let mut xoff = None;
        let mut yoff = None;
        let mut model = None;
        for (i, l) in lines[..body_start].iter().enumerate() {
            let s = strip(l);
            if let Some(c) = HEADER_TOOL.captures(s).or_else(|| HEADER_TOOL_FALLBACK.captures(s)) {
                let n = parse_uint(&c[1]).ok_or_else(|| bad(i))?;
                if !tools.iter().any(|(k, _)| *k == n) {
                    let desc = WHITESPACE.replace_all(&c[2], " ");
                    tools.push((n, strip(&desc).to_string()));
                }
            }
            if let Some(c) = ZMIN.captures(s) {
                zmin = Some(parse_float(&c[1]).ok_or_else(|| bad(i))?);
            }
            if let Some(c) = ZORIGIN.captures(s) {
                zorigin = Some(strip(&c[1]).to_string());
            }
            if let Some(c) = XOFF.captures(s) {
                xoff = Some(parse_float(&c[1]).ok_or_else(|| bad(i))?);
            }
            if let Some(c) = YOFF.captures(s) {
                yoff = Some(parse_float(&c[1]).ok_or_else(|| bad(i))?);
            }
            if model.is_none() {
                if let Some(c) = MODEL.captures(s) {
                    model = Some(strip(&c[1]).to_string());
                }
            }
        }

        let mut tool_numbers = Vec::new();
        for (i, l) in lines[body_start..body_end].iter().enumerate() {
            if let Some(c) = TOOLCHANGE.captures(l) {
                let n = parse_uint(&c[1]).ok_or_else(|| bad(body_start + i))?;
                if !tool_numbers.contains(&n) {
                    tool_numbers.push(n);
                }
            }
        }
        for &n in &tool_numbers {
            if !tools.iter().any(|(k, _)| *k == n) {
                tools.push((n, format!("Werkzeug {n}")));
            }
        }

        let code_upper = |l: &String| COMMENT.replace_all(l, "").to_uppercase();
        let prolog_code: Vec<String> = lines[..body_start].iter().map(code_upper).collect();
        let prolog_code = prolog_code.join(" ");
        let units = if G20.is_match(&prolog_code) { Units::Inch } else { Units::Mm };
        let incremental =
            G91.is_match(&prolog_code) || lines[body_start..body_end].iter().any(|l| G91.is_match(&code_upper(l)));

        Ok(NcFile {
            name: name.to_string(),
            newline,
            lines,
            body_start,
            body_end,
            tools,
            tool_numbers,
            zmin,
            zorigin,
            xoff,
            yoff,
            model,
            units,
            incremental,
        })
    }

    pub fn prolog(&self) -> &[String] {
        &self.lines[..self.body_start]
    }

    pub fn body(&self) -> &[String] {
        &self.lines[self.body_start..self.body_end]
    }

    pub fn epilog(&self) -> &[String] {
        &self.lines[self.body_end..]
    }

    pub fn tool(&self, n: u64) -> Option<&str> {
        self.tools.iter().find(|(k, _)| *k == n).map(|(_, d)| d.as_str())
    }

    pub fn tool_desc(&self) -> &str {
        match self.tool_numbers.first() {
            Some(&n) => self.tool(n).unwrap_or("?"),
            None => self.tools.first().map(|(_, d)| d.as_str()).unwrap_or("?"),
        }
    }

    pub fn is_merged(&self) -> bool {
        self.lines.first().is_some_and(|l| l.trim_start().starts_with("(Zusammengefuegt mit "))
    }

    pub fn is_stock_top(&self) -> bool {
        self.zorigin.as_deref().is_some_and(|z| z.eq_ignore_ascii_case("stock top"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> NcFile {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures").join(name);
        NcFile::read(&p).unwrap()
    }

    #[test]
    fn kopf_lesen() {
        let f = fixture("4_T16_Gravur.cnc");
        assert_eq!(f.tool_numbers, vec![1]);
        assert!(f.tool_desc().contains("Engraving"));
        assert_eq!(f.tool_desc(), "Single Flute Engraving Metal 30 deg*.3mm");
        assert_eq!(f.zmin, Some(-2.1));
        assert_eq!(f.zorigin.as_deref(), Some("Stock Top"));
        assert_eq!(f.units, Units::Mm);
        assert_eq!(f.model.as_deref(), Some("Z1 3 axis"));
        assert_eq!(f.newline, "\r\n");
        assert!(f.is_stock_top());
        assert!(!f.incremental);
    }

    #[test]
    fn mehrere_werkzeuge_und_rueckfall() {
        let f = fixture("edge/a_mehrwerkzeug.nc");
        assert_eq!(f.tool_numbers, vec![1, 2]);
        assert_eq!(f.tool(1), Some("Flat End Mill 2mm Makera"));
        assert_eq!(f.tool(2), Some("Ball Mill 1mm"));
        assert_eq!(f.newline, "\n");
        assert_eq!(f.epilog(), &["M30".to_string(), "%".to_string()]);
        assert_eq!(f.body()[0], "(Tasche)");
    }

    fn parse(s: &str) -> Result<NcFile, NcError> {
        NcFile::parse("x.nc", s.as_bytes())
    }

    #[test]
    fn fehlerfaelle() {
        assert_eq!(parse("G0 X1\nM30\n").unwrap_err(), NcError::NoToolChange { file: "x.nc".into() });
        assert_eq!(parse("T1M6\nG0 X1\n").unwrap_err(), NcError::NoProgramEnd { file: "x.nc".into() });
        assert_eq!(parse("M30\nT1M6\n").unwrap_err(), NcError::Structure { file: "x.nc".into() });
        assert!(matches!(
            parse("(   Toolpath Z Min from Stock Top: 1.2.3 mm )\nG90\nT1M6\nM30\n").unwrap_err(),
            NcError::BadNumber { line: 1, .. }
        ));
    }

    #[test]
    fn einheiten_und_inkrementell() {
        let f = parse("G20 (G21)\nT1M6\nG91\nM30\n").unwrap();
        assert_eq!(f.units, Units::Inch);
        assert!(f.incremental);
        let f = parse("(G20)\nT1M6\n(G91)\nM30\n").unwrap();
        assert_eq!(f.units, Units::Mm);
        assert!(!f.incremental);
        assert_eq!(f.tool(1), Some("Werkzeug 1"));
    }

    #[test]
    fn programmende_erkennung() {
        assert!(END.is_match("M30"));
        assert!(END.is_match("  m02"));
        assert!(END.is_match("M2 (ende)"));
        assert!(!END.is_match("M300"));
        assert!(!END.is_match("M21"));
        assert!(TOOLCHANGE.is_match("t2 m06"));
        assert!(TOOLCHANGE.is_match(" T 13 M6"));
        assert!(!TOOLCHANGE.is_match("T1M61"));
    }

    #[test]
    fn ungueltige_bytes_werden_ersetzt() {
        let f = NcFile::parse("x.nc", b"(\xff\xfe)\nT1M6\nM30\n").unwrap();
        assert_eq!(f.lines[0], "(\u{fffd}\u{fffd})");
    }
}
