use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "code", rename_all = "camelCase")]
pub enum NcError {
    Io { file: String, message: String },
    NoToolChange { file: String },
    NoProgramEnd { file: String },
    Structure { file: String },
    BadNumber { file: String, line: usize },
    NoFiles,
    MixedUnits,
}

impl fmt::Display for NcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NcError::Io { file, message } => write!(f, "{file}: Datei konnte nicht gelesen werden ({message})."),
            NcError::NoToolChange { file } => write!(f, "{file}: kein Werkzeugwechsel (T.. M6) gefunden."),
            NcError::NoProgramEnd { file } => write!(f, "{file}: kein Programmende (M30 / M2) gefunden."),
            NcError::Structure { file } => write!(f, "{file}: Aufbau nicht erkannt."),
            NcError::BadNumber { file, line } => write!(f, "{file}: Zahl in Zeile {line} nicht lesbar."),
            NcError::NoFiles => write!(f, "Keine Dateien ausgewählt."),
            NcError::MixedUnits => write!(f, "Die Dateien verwenden unterschiedliche Einheiten (mm / Zoll)."),
        }
    }
}

impl std::error::Error for NcError {}
