use multipass_core::{
    assign_tools, merge, natural_key, InventoryEntry, MergeOptions, NcError, NcFile, RapidStats, ToolMap, Units,
    Warning,
};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub number: u64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub tools: Vec<ToolInfo>,
    pub zmin: Option<f64>,
    pub zorigin: Option<String>,
    pub stock_top: bool,
    pub units: Units,
    pub incremental: bool,
    pub lines: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadError {
    pub name: String,
    pub path: String,
    pub error: NcError,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Prepared {
    pub warnings: Vec<Warning>,
    pub warning_texts: Vec<String>,
    pub rapid: Option<RapidStats>,
    pub suggested_dir: Option<String>,
    pub suggested_name: String,
    pub lines: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Saved {
    pub path: String,
    pub dir: Option<String>,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub error: Option<NcError>,
    pub message: String,
}

impl From<NcError> for CommandError {
    fn from(e: NcError) -> Self {
        CommandError { message: e.to_string(), error: Some(e) }
    }
}

impl CommandError {
    pub fn plain(message: impl Into<String>) -> Self {
        CommandError { error: None, message: message.into() }
    }
}

struct Loaded {
    path: PathBuf,
    file: Arc<NcFile>,
}

#[derive(Default)]
pub struct AppState {
    files: Mutex<HashMap<u64, Loaded>>,
    next_id: Mutex<u64>,
    prepared: Mutex<Option<String>>,
}

pub fn sort_paths(paths: &mut [PathBuf]) {
    paths.sort_by_key(|p| natural_key(&p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
}

impl AppState {
    pub fn insert(&self, path: PathBuf, file: NcFile) -> FileInfo {
        let id = {
            let mut n = self.next_id.lock().unwrap();
            *n += 1;
            *n
        };
        let info = FileInfo {
            id,
            name: file.name.clone(),
            path: path.to_string_lossy().into_owned(),
            tools: file
                .tool_numbers
                .iter()
                .map(|&n| ToolInfo { number: n, description: file.tool(n).unwrap_or("").to_string() })
                .collect(),
            zmin: file.zmin,
            zorigin: file.zorigin.clone(),
            stock_top: file.is_stock_top(),
            units: file.units,
            incremental: file.incremental,
            lines: file.lines.len(),
        };
        self.files.lock().unwrap().insert(id, Loaded { path, file: Arc::new(file) });
        info
    }

    pub fn forget(&self, ids: &[u64]) {
        let mut files = self.files.lock().unwrap();
        for id in ids {
            files.remove(id);
        }
    }

    fn collect(&self, ids: &[u64]) -> Result<Vec<(PathBuf, Arc<NcFile>)>, CommandError> {
        let files = self.files.lock().unwrap();
        ids.iter()
            .map(|id| {
                files
                    .get(id)
                    .map(|l| (l.path.clone(), l.file.clone()))
                    .ok_or_else(|| CommandError::plain(format!("Datei {id} ist nicht mehr geladen.")))
            })
            .collect()
    }

    pub fn auto_tools(&self, ids: &[u64], inventory: &[InventoryEntry]) -> Result<Vec<ToolMap>, CommandError> {
        let files: Vec<Arc<NcFile>> = self.collect(ids)?.into_iter().map(|(_, f)| f).collect();
        Ok(assign_tools(&files, inventory))
    }

    pub fn prepare(
        &self,
        ids: &[u64],
        maps: &[ToolMap],
        restore: bool,
        safe_z: f64,
        stamp: String,
    ) -> Result<Prepared, CommandError> {
        let loaded = self.collect(ids)?;
        if maps.len() != loaded.len() {
            return Err(CommandError::plain("Werkzeugzuordnung passt nicht zur Dateiliste."));
        }
        if !(0.5..=20.0).contains(&safe_z) {
            return Err(CommandError::plain("Sicherheitshöhe muss zwischen 0,5 und 20 mm liegen."));
        }
        let files: Vec<Arc<NcFile>> = loaded.iter().map(|(_, f)| f.clone()).collect();
        let result = merge(&files, maps, &MergeOptions { restore, safe_z, stamp })?;
        let first = &loaded[0].0;
        let ext = first
            .extension()
            .map(|e| e.to_string_lossy().into_owned())
            .filter(|e| !e.is_empty())
            .unwrap_or_else(|| "cnc".into());
        let prepared = Prepared {
            warning_texts: result.warnings.iter().map(|w| w.to_string()).collect(),
            warnings: result.warnings,
            rapid: result.rapid,
            suggested_dir: first.parent().map(|p| p.to_string_lossy().into_owned()),
            suggested_name: format!("GESAMT.{ext}"),
            lines: result.text.lines().count(),
        };
        *self.prepared.lock().unwrap() = Some(result.text);
        Ok(prepared)
    }

    pub fn save(&self, path: &Path) -> Result<Saved, CommandError> {
        let text = self
            .prepared
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| CommandError::plain("Es wurde noch nichts zusammengefügt."))?;
        let file_name = path.file_name().ok_or_else(|| CommandError::plain("Ungültiger Dateiname."))?;
        let mut tmp_name = file_name.to_os_string();
        tmp_name.push(".multipass-tmp");
        let tmp = path.with_file_name(tmp_name);
        let fail = |e: std::io::Error| {
            CommandError::plain(format!("{}: konnte nicht gespeichert werden ({e}).", path.display()))
        };
        std::fs::write(&tmp, text.as_bytes()).map_err(fail)?;
        if let Err(e) = std::fs::rename(&tmp, path) {
            let _ = std::fs::remove_file(&tmp);
            return Err(fail(e));
        }
        Ok(Saved {
            path: path.to_string_lossy().into_owned(),
            dir: path.parent().map(|p| p.to_string_lossy().into_owned()),
            bytes: text.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures").join(name)
    }

    #[test]
    fn laden_zuordnen_zusammenfuegen_speichern() {
        let st = AppState::default();
        let mut paths = vec![fixture("5_T13_Kontur.cnc"), fixture("4_T16_Gravur.cnc")];
        sort_paths(&mut paths);
        let infos: Vec<FileInfo> = paths.iter().map(|p| st.insert(p.clone(), NcFile::read(p).unwrap())).collect();
        assert_eq!(infos[0].name, "4_T16_Gravur.cnc");
        assert!(infos[0].stock_top);
        let ids: Vec<u64> = infos.iter().map(|i| i.id).collect();
        let maps = st.auto_tools(&ids, &[]).unwrap();
        assert_eq!(maps[1][&1], 2);
        let p = st.prepare(&ids, &maps, true, 1.0, "x".into()).unwrap();
        assert!(p.warnings.is_empty());
        assert!(p.rapid.unwrap().converted > 0);
        assert_eq!(p.suggested_name, "GESAMT.cnc");
        let dir = std::env::temp_dir().join(format!("multipass_app_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("GESAMT.cnc");
        std::fs::write(&out, "alt").unwrap();
        let s = st.save(&out).unwrap();
        let written = std::fs::read_to_string(&out).unwrap();
        assert_eq!(written.len(), s.bytes);
        assert!(written.ends_with("M30\r\n"));
        assert!(!dir.join("GESAMT.cnc.multipass-tmp").exists());
        std::fs::remove_dir_all(&dir).unwrap();
        st.forget(&ids[..1]);
        assert!(st.prepare(&ids, &maps, false, 1.0, "x".into()).is_err());
    }

    #[test]
    fn sicherheitshoehe_wird_geprueft() {
        let st = AppState::default();
        let p = fixture("4_T16_Gravur.cnc");
        let id = st.insert(p.clone(), NcFile::read(&p).unwrap()).id;
        assert!(st.prepare(&[id], &[ToolMap::new()], true, 0.1, "x".into()).is_err());
        assert!(st.save(Path::new("x.cnc")).is_err());
    }
}
