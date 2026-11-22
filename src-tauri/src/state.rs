use multipass_core::{
    assign_tools, effective_body, encode_timeline, estimate_file, merge, natural_key, preview, rapid_allowed, timeline,
    write_atomic, Estimate, InventoryEntry, MergeOptions, NcError, NcFile, Preview, RapidStats, ToolMap, Units,
    Warning, SAFE_Z_MAX, SAFE_Z_MIN,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEstimate {
    pub id: u64,
    pub plain: Estimate,
    pub fast: Option<Estimate>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimateOutcome {
    pub rapid_allowed: bool,
    pub files: Vec<FileEstimate>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePreview {
    pub id: u64,
    pub preview: Preview,
}

type EstimateKey = (u64, Option<u64>);

struct Loaded {
    path: PathBuf,
    file: Arc<NcFile>,
}

#[derive(Default)]
pub struct AppState {
    files: Mutex<HashMap<u64, Loaded>>,
    next_id: Mutex<u64>,
    prepared: Mutex<Option<String>>,
    estimates: Mutex<HashMap<EstimateKey, Estimate>>,
}

pub fn clamp_safe_z(z: f64) -> f64 {
    if z.is_finite() {
        z.clamp(SAFE_Z_MIN, SAFE_Z_MAX)
    } else {
        1.0
    }
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
        self.estimates.lock().unwrap().retain(|k, _| !ids.contains(&k.0));
    }

    fn collect(&self, ids: &[u64]) -> Result<Vec<(PathBuf, Arc<NcFile>)>, CommandError> {
        let files = self.files.lock().unwrap();
        ids.iter()
            .map(|id| {
                files
                    .get(id)
                    .map(|l| (l.path.clone(), l.file.clone()))
                    .ok_or_else(|| CommandError::plain(format!("File {id} is no longer loaded.")))
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
        fan_pause: bool,
        stamp: String,
    ) -> Result<Prepared, CommandError> {
        let loaded = self.collect(ids)?;
        if maps.len() != loaded.len() {
            return Err(CommandError::plain("Tool assignment does not match the file list."));
        }
        if !(SAFE_Z_MIN..=SAFE_Z_MAX).contains(&safe_z) {
            return Err(CommandError::plain("Safe height must be between 0.5 and 20 mm."));
        }
        let files: Vec<Arc<NcFile>> = loaded.iter().map(|(_, f)| f.clone()).collect();
        let result = merge(&files, maps, &MergeOptions { restore, safe_z, stamp, fan_pause })?;
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
            suggested_name: format!("MERGED.{ext}"),
            lines: result.text.lines().count(),
        };
        *self.prepared.lock().unwrap() = Some(result.text);
        Ok(prepared)
    }

    pub fn estimate(&self, ids: &[u64], safe_z: f64) -> Result<EstimateOutcome, CommandError> {
        let loaded = self.collect(ids)?;
        let files: Vec<Arc<NcFile>> = loaded.into_iter().map(|(_, f)| f).collect();
        let allowed = rapid_allowed(&files);
        let fast_key = allowed.then(|| clamp_safe_z(safe_z).to_bits());
        let mut wanted: Vec<(usize, EstimateKey)> = Vec::new();
        for (i, id) in ids.iter().enumerate() {
            wanted.push((i, (*id, None)));
            if let Some(k) = fast_key {
                wanted.push((i, (*id, Some(k))));
            }
        }
        let missing: Vec<(usize, EstimateKey)> = {
            let cache = self.estimates.lock().unwrap();
            wanted.iter().filter(|(_, k)| !cache.contains_key(k)).copied().collect()
        };
        let computed: Vec<(EstimateKey, Estimate)> = std::thread::scope(|s| {
            let handles: Vec<_> = missing
                .iter()
                .map(|&(i, key)| {
                    let f = files[i].clone();
                    s.spawn(move || (key, estimate_file(&f, key.1.map(f64::from_bits))))
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        let mut cache = self.estimates.lock().unwrap();
        cache.extend(computed);
        let out = ids
            .iter()
            .map(|id| FileEstimate {
                id: *id,
                plain: cache.get(&(*id, None)).copied().unwrap_or_default(),
                fast: fast_key.and_then(|k| cache.get(&(*id, Some(k))).copied()),
            })
            .collect();
        Ok(EstimateOutcome { rapid_allowed: allowed, files: out })
    }

    pub fn preview(&self, ids: &[u64], restore: bool, safe_z: f64) -> Result<Vec<FilePreview>, CommandError> {
        let loaded = self.collect(ids)?;
        let files: Vec<Arc<NcFile>> = loaded.into_iter().map(|(_, f)| f).collect();
        let z = (restore && rapid_allowed(&files)).then(|| clamp_safe_z(safe_z));
        let budget = (120_000 / files.len().max(1)).max(8_000);
        Ok(std::thread::scope(|s| {
            let handles: Vec<_> = files
                .iter()
                .zip(ids)
                .map(|(f, id)| {
                    s.spawn(move || FilePreview {
                        id: *id,
                        preview: preview(&effective_body(f, z), f.units == Units::Inch, budget),
                    })
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        }))
    }

    pub fn timeline(&self, ids: &[u64], restore: bool, safe_z: f64) -> Result<Vec<u8>, CommandError> {
        let loaded = self.collect(ids)?;
        let files: Vec<Arc<NcFile>> = loaded.into_iter().map(|(_, f)| f).collect();
        let z = (restore && rapid_allowed(&files)).then(|| clamp_safe_z(safe_z));
        Ok(encode_timeline(&timeline(&files, z)))
    }

    pub fn save(&self, path: &Path) -> Result<Saved, CommandError> {
        let text =
            self.prepared.lock().unwrap().clone().ok_or_else(|| CommandError::plain("Nothing has been merged yet."))?;
        path.file_name().ok_or_else(|| CommandError::plain("Invalid file name."))?;
        write_atomic(path, text.as_bytes())
            .map_err(|e| CommandError::plain(format!("{}: could not be saved ({e}).", path.display())))?;
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
    fn load_assign_merge_save() {
        let st = AppState::default();
        let mut paths = vec![fixture("5_T13_Kontur.cnc"), fixture("4_T16_Gravur.cnc")];
        sort_paths(&mut paths);
        let infos: Vec<FileInfo> = paths.iter().map(|p| st.insert(p.clone(), NcFile::read(p).unwrap())).collect();
        assert_eq!(infos[0].name, "4_T16_Gravur.cnc");
        assert!(infos[0].stock_top);
        let ids: Vec<u64> = infos.iter().map(|i| i.id).collect();
        let maps = st.auto_tools(&ids, &[]).unwrap();
        assert_eq!(maps[1][&1], 2);
        let p = st.prepare(&ids, &maps, true, 1.0, false, "x".into()).unwrap();
        assert!(p.warnings.is_empty());
        assert!(p.rapid.unwrap().converted > 0);
        assert_eq!(p.suggested_name, "MERGED.cnc");
        let dir = std::env::temp_dir().join(format!("multipass_app_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("MERGED.cnc");
        std::fs::write(&out, "old").unwrap();
        let s = st.save(&out).unwrap();
        let written = std::fs::read_to_string(&out).unwrap();
        assert_eq!(written.len(), s.bytes);
        assert!(written.ends_with("M30\r\n"));
        assert!(!dir.join("MERGED.cnc.multipass-tmp").exists());
        std::fs::remove_dir_all(&dir).unwrap();
        st.forget(&ids[..1]);
        assert!(st.prepare(&ids, &maps, false, 1.0, false, "x".into()).is_err());
    }

    #[test]
    fn safe_height_is_checked() {
        let st = AppState::default();
        let p = fixture("4_T16_Gravur.cnc");
        let id = st.insert(p.clone(), NcFile::read(&p).unwrap()).id;
        assert!(st.prepare(&[id], &[ToolMap::new()], true, 0.1, false, "x".into()).is_err());
        assert!(st.save(Path::new("x.cnc")).is_err());
    }
}
