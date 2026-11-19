use multipass_core::{default_tool_map, merge, MergeOptions, NcFile};
use std::path::PathBuf;
use std::process::ExitCode;

const HILFE: &str = "Aufruf: multipass-cli DATEI... [-o AUSGABE] [--rapid] [--safe-z MM] [--stamp TEXT]

  -o AUSGABE     Zieldatei (Standard: GESAMT.cnc)
  --rapid        Leerfahrten ueber dem Material wieder als Eilgang (G0)
  --safe-z MM    Sicherheitshoehe fuer --rapid (Standard: 1.0)
  --stamp TEXT   fester Zeitstempel statt aktueller Zeit";

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut inputs: Vec<PathBuf> = Vec::new();
    let mut output = PathBuf::from("GESAMT.cnc");
    let mut restore = false;
    let mut safe_z = 1.0;
    let mut stamp = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("{HILFE}");
                return Ok(());
            }
            "-o" => output = args.next().ok_or("-o braucht einen Dateinamen")?.into(),
            "--rapid" => restore = true,
            "--safe-z" => {
                let v = args.next().ok_or("--safe-z braucht einen Wert")?;
                safe_z = v.replace(',', ".").parse().map_err(|_| format!("--safe-z: '{v}' ist keine Zahl"))?;
            }
            "--stamp" => stamp = Some(args.next().ok_or("--stamp braucht einen Text")?),
            _ => inputs.push(a.into()),
        }
    }
    if inputs.is_empty() {
        return Err(HILFE.to_string());
    }
    let files = inputs.iter().map(|p| NcFile::read(p)).collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    let stamp = stamp.unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d %H:%M").to_string());
    let result = merge(&files, &default_tool_map(&files), &MergeOptions { restore, safe_z, stamp })
        .map_err(|e| e.to_string())?;
    std::fs::write(&output, result.text.as_bytes())
        .map_err(|e| format!("{}: konnte nicht geschrieben werden ({e})", output.display()))?;
    println!("{}", result.report.join("\n"));
    for w in &result.warnings {
        println!("HINWEIS: {w}");
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("FEHLER: {e}");
            ExitCode::FAILURE
        }
    }
}
