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
    AlreadyMerged { file: String },
    NoFiles,
    MixedUnits,
}

impl fmt::Display for NcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NcError::Io { file, message } => write!(f, "{file}: could not read the file ({message})."),
            NcError::NoToolChange { file } => write!(f, "{file}: no tool change (T.. M6) found."),
            NcError::NoProgramEnd { file } => write!(f, "{file}: no program end (M30 / M2) found."),
            NcError::Structure { file } => write!(f, "{file}: file structure not recognised."),
            NcError::BadNumber { file, line } => write!(f, "{file}: unreadable number in line {line}."),
            NcError::AlreadyMerged { file } => write!(
                f,
                "{file}: already merged by MultiPass and was skipped, otherwise everything would be milled twice."
            ),
            NcError::NoFiles => write!(f, "No files selected."),
            NcError::MixedUnits => write!(f, "The files use different units (mm / inch)."),
        }
    }
}

impl std::error::Error for NcError {}
